//! Tauri IPC command handlers for SuperBrain

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::state::{AppSettings, AppState, SystemStatus};

// ---- Think / Chat ----

#[derive(Debug, Serialize, Deserialize)]
pub struct ThinkResponse {
    pub response: String,
    pub confidence: f64,
    pub thought_id: String,
    pub memory_count: u32,
    pub ai_enhanced: bool,
}

#[tauri::command]
pub async fn think(input: String, state: State<'_, AppState>) -> Result<ThinkResponse, String> {
    let embedding = state.embeddings.embed(&input).await?;

    // First, get memory-based response
    let brain_result = state.engine.think_with_embedding(&input, &embedding)?;

    Ok(ThinkResponse {
        response: brain_result.response,
        confidence: brain_result.confidence,
        thought_id: brain_result.thought_id,
        memory_count: brain_result.memory_count,
        ai_enhanced: false,
    })
}

// ---- Remember ----

#[derive(Debug, Serialize, Deserialize)]
pub struct RememberResponse {
    pub id: String,
    pub memory_count: u32,
}

#[tauri::command]
pub async fn remember(
    content: String,
    memory_type: String,
    importance: Option<f64>,
    state: State<'_, AppState>,
) -> Result<RememberResponse, String> {
    let embedding = state.embeddings.embed(&content).await?;

    let id = state.engine.remember_with_embedding(
        content,
        embedding,
        memory_type,
        importance,
    )?;

    // Persist to disk
    if let Some(node) = {
        // Get the node we just stored
        let nodes = state.engine.memory.all_nodes();
        nodes.into_iter().find(|n| n.id == id)
    } {
        let _ = state.persistence.store_memory(&node);
    }

    let memory_count = state.engine.memory.len();

    Ok(RememberResponse { id, memory_count })
}

// ---- Recall ----

#[derive(Debug, Serialize, Deserialize)]
pub struct RecallItem {
    pub id: String,
    pub content: String,
    pub similarity: f64,
    pub memory_type: String,
}

#[tauri::command]
pub async fn recall(
    query: String,
    limit: Option<u32>,
    state: State<'_, AppState>,
) -> Result<Vec<RecallItem>, String> {
    let embedding = state.embeddings.embed(&query).await?;

    let results = state
        .engine
        .recall_f32(&embedding, limit, None)?;

    Ok(results
        .into_iter()
        .map(|r| RecallItem {
            id: r.id,
            content: r.content,
            similarity: r.similarity,
            memory_type: r.memory_type,
        })
        .collect())
}

// ---- Status ----

#[tauri::command]
pub fn get_status(state: State<'_, AppState>) -> Result<SystemStatus, String> {
    let introspection = state.engine.introspect();
    let settings = state.settings.read();
    let embedding_provider = format!("{:?}", state.embeddings.provider());

    let index_stats = state.indexer.stats().unwrap_or(crate::indexer::IndexStats {
        file_count: 0,
        chunk_count: 0,
        watched_dirs: 0,
        is_indexing: false,
    });

    Ok(SystemStatus {
        status: introspection.status,
        memory_count: introspection.total_memories,
        thought_count: introspection.total_thoughts,
        uptime_ms: introspection.uptime_ms,
        ai_provider: settings.ai_provider.clone(),
        ai_available: true,
        embedding_provider,
        learning_trend: introspection.learning_trend,
        indexed_files: index_stats.file_count,
        indexed_chunks: index_stats.chunk_count,
    })
}

// ---- Settings ----

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    Ok(state.settings.read().clone())
}

#[tauri::command]
pub fn update_settings(
    settings: AppSettings,
    state: State<'_, AppState>,
) -> Result<(), String> {
    *state.settings.write() = settings.clone();

    // Persist
    let json = serde_json::to_string(&settings).map_err(|e| format!("Serialize error: {}", e))?;
    state.persistence.store_config("app_settings", &json)?;

    Ok(())
}

// ---- Thoughts ----

#[tauri::command]
pub fn get_thoughts(
    limit: Option<u32>,
    state: State<'_, AppState>,
) -> Result<Vec<crate::brain::types::Thought>, String> {
    Ok(state.engine.get_thoughts(limit))
}

// ---- Stats ----

#[tauri::command]
pub fn get_stats(
    state: State<'_, AppState>,
) -> Result<crate::brain::types::CognitiveStats, String> {
    Ok(state.engine.stats())
}

// ---- Evolve ----

#[tauri::command]
pub fn evolve(
    state: State<'_, AppState>,
) -> Result<crate::brain::cognitive::EvolutionResult, String> {
    Ok(state.engine.evolve())
}

// ---- Cycle ----

#[tauri::command]
pub fn cycle(
    state: State<'_, AppState>,
) -> Result<crate::brain::cognitive::CycleResult, String> {
    Ok(state.engine.cycle())
}

// ---- File Search ----

#[tauri::command]
pub async fn search_files(
    query: String,
    limit: Option<u32>,
    state: State<'_, AppState>,
) -> Result<Vec<crate::indexer::FileResult>, String> {
    state.indexer.search(&query, limit.unwrap_or(10)).await
}

// ---- Index Files ----

#[tauri::command]
pub async fn index_files(state: State<'_, AppState>) -> Result<u32, String> {
    state.indexer.scan_all().await
}

// ---- Workflows ----

#[tauri::command]
pub async fn run_workflow(
    action: String,
    query: Option<String>,
    state: State<'_, AppState>,
) -> Result<crate::workflows::WorkflowResult, String> {
    let workflow_action = match action.as_str() {
        "remember_clipboard" => crate::workflows::WorkflowAction::RememberClipboard,
        "summarize" => crate::workflows::WorkflowAction::SummarizeRecent,
        "digest" => crate::workflows::WorkflowAction::LearningDigest,
        "search_and_remember" => crate::workflows::WorkflowAction::SearchAndRemember {
            query: query.unwrap_or_default(),
        },
        _ => return Err(format!("Unknown workflow: {}", action)),
    };

    crate::workflows::execute_workflow(
        workflow_action,
        &state.engine,
        &state.embeddings,
        &state.context,
    )
    .await
}

// ---- Flush (save to disk) ----

#[tauri::command]
pub fn flush(state: State<'_, AppState>) -> Result<(), String> {
    state.flush()
}
