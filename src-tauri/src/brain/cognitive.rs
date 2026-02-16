//! Cognitive processing engine for SuperBrain (Tauri port)

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::brain::learning::NativeLearner;
use crate::brain::memory::NativeMemory;
use crate::brain::types::{CognitiveConfig, CognitiveStats, Thought, ThoughtType};
use crate::brain::utils::{generate_id, now_millis};

/// Goal tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Goal {
    pub id: String,
    pub description: String,
    pub priority: f64,
    pub progress: f64,
    pub status: GoalStatus,
    pub created_at: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(crate) enum GoalStatus {
    Pending,
    Active,
    Completed,
    Failed,
}

/// Belief with confidence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Belief {
    pub id: String,
    pub content: String,
    pub confidence: f64,
    pub source: String,
    pub timestamp: i64,
}

/// The main cognitive engine
pub struct CognitiveEngine {
    /// Native memory system
    pub memory: Arc<NativeMemory>,
    /// Native learner
    pub learner: Arc<NativeLearner>,
    /// Thought stream
    thoughts: RwLock<Vec<Thought>>,
    /// Goals
    goals: RwLock<Vec<Goal>>,
    /// Beliefs
    beliefs: RwLock<Vec<Belief>>,
    /// Configuration
    config: RwLock<CognitiveConfig>,
    /// Running state
    running: AtomicBool,
    /// Cycle counter
    cycle_count: AtomicU64,
    /// Start time
    start_time: i64,
}

impl CognitiveEngine {
    /// Create a new cognitive engine
    pub fn new(config: Option<CognitiveConfig>) -> Self {
        let cfg = config.unwrap_or_default();
        let dimensions = cfg.dimensions;
        let action_count = 100;

        Self {
            memory: Arc::new(NativeMemory::new(dimensions)),
            learner: Arc::new(NativeLearner::new(dimensions, action_count)),
            thoughts: RwLock::new(Vec::with_capacity(1000)),
            goals: RwLock::new(Vec::new()),
            beliefs: RwLock::new(Vec::new()),
            config: RwLock::new(cfg),
            running: AtomicBool::new(false),
            cycle_count: AtomicU64::new(0),
            start_time: now_millis(),
        }
    }

    /// Store a memory with a text embedding (uses f32 vectors directly)
    pub fn remember_with_embedding(
        &self,
        content: String,
        vector: Vec<f32>,
        memory_type: String,
        importance: Option<f64>,
    ) -> Result<String, String> {
        let imp = importance.unwrap_or(0.5);
        self.memory
            .store_f32(content, vector, memory_type, imp)
    }

    /// Store a memory (legacy f64 interface)
    pub fn remember(
        &self,
        content: String,
        vector: Vec<f64>,
        memory_type: String,
        importance: Option<f64>,
    ) -> Result<String, String> {
        let imp = importance.unwrap_or(0.5);
        self.memory.store(content, vector, memory_type, imp)
    }

    /// Recall memories by similarity (f32 interface)
    pub fn recall_f32(
        &self,
        query_vector: &[f32],
        k: Option<u32>,
        memory_types: Option<Vec<String>>,
    ) -> Result<Vec<RecallResult>, String> {
        let results =
            self.memory
                .search_f32(query_vector, k.unwrap_or(10), memory_types, Some(0.2))?;

        Ok(results
            .into_iter()
            .map(|r| RecallResult {
                id: r.id,
                content: r.content,
                similarity: r.similarity,
                memory_type: r.memory_type,
            })
            .collect())
    }

    /// Recall memories by similarity (legacy f64 interface)
    pub fn recall(
        &self,
        query_vector: Vec<f64>,
        k: Option<u32>,
        memory_types: Option<Vec<String>>,
    ) -> Result<Vec<RecallResult>, String> {
        let results = self
            .memory
            .search(query_vector, k.unwrap_or(10), memory_types, Some(0.2))?;

        Ok(results
            .into_iter()
            .map(|r| RecallResult {
                id: r.id,
                content: r.content,
                similarity: r.similarity,
                memory_type: r.memory_type,
            })
            .collect())
    }

    /// Learn from an experience
    pub fn learn(
        &self,
        state: Vec<f64>,
        action: u32,
        reward: f64,
        next_state: Vec<f64>,
        done: bool,
    ) -> Result<LearnResult, String> {
        use crate::brain::types::Experience;

        let experience = Experience {
            state,
            action,
            reward,
            next_state,
            done,
        };

        let outcome = self.learner.learn(experience)?;

        let thought = self.generate_thought(
            if outcome.success {
                ThoughtType::Evaluation
            } else {
                ThoughtType::Reflection
            },
            format!(
                "Learned from experience: reward={:.2}, td_error={:.2}",
                outcome.reward, outcome.td_error
            ),
            outcome.reward.abs().min(1.0),
        );

        Ok(LearnResult {
            success: outcome.success,
            reward: outcome.reward,
            td_error: outcome.td_error,
            thought_id: thought.id,
            insights: outcome.insights,
        })
    }

    /// Select an action for a given state
    pub fn act(&self, state: Vec<f64>) -> u32 {
        self.learner.select_action(state)
    }

    /// Think - process input and generate response (with pre-computed embedding)
    pub fn think_with_embedding(
        &self,
        input: &str,
        embedding: &[f32],
    ) -> Result<ThinkResult, String> {
        let memories = self.recall_f32(embedding, Some(5), None)?;

        let thought = self.generate_thought(
            ThoughtType::Inference,
            format!("Processing: {}", &input[..input.len().min(100)]),
            0.7,
        );

        let response = if memories.is_empty() {
            "No relevant information found in memory.".to_string()
        } else {
            format!(
                "Based on {} relevant memories: {}",
                memories.len(),
                memories
                    .first()
                    .map(|m| m.content.clone())
                    .unwrap_or_default()
            )
        };

        let confidence = memories.first().map(|m| m.similarity).unwrap_or(0.1);

        Ok(ThinkResult {
            response,
            confidence,
            thought_id: thought.id,
            memory_count: memories.len() as u32,
        })
    }

    /// Think - process input and generate response (legacy f64 interface)
    pub fn think(&self, input: String, input_vector: Vec<f64>) -> Result<ThinkResult, String> {
        let memories = self.recall(input_vector.clone(), Some(5), None)?;

        let thought = self.generate_thought(
            ThoughtType::Inference,
            format!("Processing: {}", &input[..input.len().min(100)]),
            0.7,
        );

        let response = if memories.is_empty() {
            "No relevant information found in memory.".to_string()
        } else {
            format!(
                "Based on {} relevant memories: {}",
                memories.len(),
                memories
                    .first()
                    .map(|m| m.content.clone())
                    .unwrap_or_default()
            )
        };

        let confidence = memories.first().map(|m| m.similarity).unwrap_or(0.1);

        Ok(ThinkResult {
            response,
            confidence,
            thought_id: thought.id,
            memory_count: memories.len() as u32,
        })
    }

    /// Add a goal
    pub fn add_goal(&self, description: String, priority: f64) -> String {
        let goal = Goal {
            id: generate_id(),
            description,
            priority,
            progress: 0.0,
            status: GoalStatus::Pending,
            created_at: now_millis(),
        };

        let id = goal.id.clone();
        self.goals.write().push(goal);
        id
    }

    /// Update goal progress
    pub fn update_goal(&self, goal_id: &str, progress: f64) -> bool {
        let mut goals = self.goals.write();
        if let Some(goal) = goals.iter_mut().find(|g| g.id == goal_id) {
            goal.progress = progress.min(1.0);
            if goal.progress >= 1.0 {
                goal.status = GoalStatus::Completed;
            } else if goal.progress > 0.0 {
                goal.status = GoalStatus::Active;
            }
            true
        } else {
            false
        }
    }

    /// Add a belief
    pub fn add_belief(&self, content: String, confidence: f64, source: String) -> String {
        let belief = Belief {
            id: generate_id(),
            content,
            confidence,
            source,
            timestamp: now_millis(),
        };

        let id = belief.id.clone();
        self.beliefs.write().push(belief);
        id
    }

    /// Generate a thought
    fn generate_thought(
        &self,
        thought_type: ThoughtType,
        content: String,
        confidence: f64,
    ) -> Thought {
        let thought = Thought {
            id: generate_id(),
            content,
            thought_type: format!("{:?}", thought_type),
            confidence,
            novelty: 0.5,
            utility: confidence,
            timestamp: now_millis(),
        };

        let mut thoughts = self.thoughts.write();
        thoughts.push(thought.clone());

        if thoughts.len() > 1000 {
            thoughts.drain(0..500);
        }

        thought
    }

    /// Self-improve - analyze and adapt
    pub fn evolve(&self) -> EvolutionResult {
        let mut adaptations = Vec::new();
        let improvements = Vec::new();

        let learner_stats = self.learner.stats();

        if learner_stats.trend < -0.05 {
            self.learner.explore();
            adaptations.push("Increased exploration due to declining performance".to_string());
        }

        let memory_stats = self.memory.stats();

        if memory_stats.avg_decay > 0.5 {
            let result = self.memory.consolidate();
            adaptations.push(format!(
                "Consolidated memory: pruned {} entries",
                result.pruned
            ));
        }

        let thought = self.generate_thought(
            ThoughtType::Reflection,
            format!(
                "Self-analysis: {} memories, {} q-states, trend={:.3}",
                memory_stats.total_memories, learner_stats.q_table_size, learner_stats.trend
            ),
            0.8,
        );

        self.cycle_count.fetch_add(1, Ordering::Relaxed);

        EvolutionResult {
            adaptations,
            improvements,
            thought_id: thought.id,
        }
    }

    /// Introspect - get internal state
    pub fn introspect(&self) -> IntrospectionResult {
        let memory_stats = self.memory.stats();
        let learner_stats = self.learner.stats();
        let thoughts = self.thoughts.read();
        let goals = self.goals.read();

        let active_goals = goals
            .iter()
            .filter(|g| g.status == GoalStatus::Active || g.status == GoalStatus::Pending)
            .count() as u32;

        let trend = if learner_stats.trend > 0.05 {
            "improving"
        } else if learner_stats.trend < -0.05 {
            "declining"
        } else {
            "stable"
        };

        IntrospectionResult {
            status: "healthy".to_string(),
            uptime_ms: now_millis() - self.start_time,
            total_memories: memory_stats.total_memories,
            total_thoughts: thoughts.len() as u32,
            total_experiences: learner_stats.total_experiences as u32,
            active_goals,
            avg_reward: learner_stats.avg_reward,
            learning_trend: trend.to_string(),
            exploration_rate: learner_stats.exploration_rate,
        }
    }

    /// Get statistics
    pub fn stats(&self) -> CognitiveStats {
        let memory_stats = self.memory.stats();
        let learner_stats = self.learner.stats();

        CognitiveStats {
            total_memories: memory_stats.total_memories,
            total_thoughts: self.thoughts.read().len() as u32,
            total_experiences: learner_stats.total_experiences as u32,
            avg_importance: memory_stats.avg_importance,
            avg_reward: learner_stats.avg_reward,
            learning_trend: learner_stats.trend,
        }
    }

    /// Get recent thoughts
    pub fn get_thoughts(&self, limit: Option<u32>) -> Vec<Thought> {
        let thoughts = self.thoughts.read();
        let n = limit.unwrap_or(10) as usize;
        thoughts.iter().rev().take(n).cloned().collect()
    }

    /// Run a cognitive cycle
    pub fn cycle(&self) -> CycleResult {
        self.cycle_count.fetch_add(1, Ordering::Relaxed);

        let train_result = self.learner.train_batch().ok();
        let insights = train_result.unwrap_or_default();

        let consolidated = if self.cycle_count.load(Ordering::Relaxed) % 100 == 0 {
            Some(self.memory.consolidate())
        } else {
            None
        };

        CycleResult {
            cycle_number: self.cycle_count.load(Ordering::Relaxed),
            training_insights: insights,
            memories_pruned: consolidated.map(|c| c.pruned).unwrap_or(0),
        }
    }

    /// Set running state
    pub fn set_running(&self, running: bool) {
        self.running.store(running, Ordering::Relaxed);
    }

    /// Check if running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }
}

/// Result of memory recall
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecallResult {
    pub id: String,
    pub content: String,
    pub similarity: f64,
    pub memory_type: String,
}

/// Result of learning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnResult {
    pub success: bool,
    pub reward: f64,
    pub td_error: f64,
    pub thought_id: String,
    pub insights: Vec<String>,
}

/// Result of thinking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkResult {
    pub response: String,
    pub confidence: f64,
    pub thought_id: String,
    pub memory_count: u32,
}

/// Result of evolution/self-improvement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionResult {
    pub adaptations: Vec<String>,
    pub improvements: Vec<String>,
    pub thought_id: String,
}

/// Result of introspection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntrospectionResult {
    pub status: String,
    pub uptime_ms: i64,
    pub total_memories: u32,
    pub total_thoughts: u32,
    pub total_experiences: u32,
    pub active_goals: u32,
    pub avg_reward: f64,
    pub learning_trend: String,
    pub exploration_rate: f64,
}

/// Result of a cognitive cycle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycleResult {
    pub cycle_number: u64,
    pub training_insights: Vec<String>,
    pub memories_pruned: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cognitive_engine() {
        let engine = CognitiveEngine::new(None);

        let id = engine
            .remember(
                "Test memory".to_string(),
                vec![0.1; 384],
                "semantic".to_string(),
                Some(0.8),
            )
            .unwrap();
        assert!(!id.is_empty());

        let goal_id = engine.add_goal("Test goal".to_string(), 0.9);
        assert!(!goal_id.is_empty());

        let state = engine.introspect();
        assert_eq!(state.status, "healthy");
        assert!(state.total_memories >= 1);
    }
}
