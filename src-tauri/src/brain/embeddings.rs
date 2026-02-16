//! Embedding model for SuperBrain
//!
//! Supports:
//! - ONNX all-MiniLM-L6-v2 (384-dim, local, fast)
//! - Ollama embeddings API (fallback)
//! - Simple hash-based embeddings (ultimate fallback)

use std::path::PathBuf;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::brain::utils::normalize_vector;

const EMBEDDING_DIM: usize = 384;

/// Embedding provider type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmbeddingProvider {
    Onnx,
    Ollama,
    Hash,
}

/// Embedding model manager
pub struct EmbeddingModel {
    provider: RwLock<EmbeddingProvider>,
    ollama_url: String,
    ollama_model: String,
    _model_path: Option<PathBuf>,
}

impl EmbeddingModel {
    /// Create a new embedding model (starts with hash fallback, can be upgraded)
    pub fn new() -> Self {
        Self {
            provider: RwLock::new(EmbeddingProvider::Hash),
            ollama_url: "http://localhost:11434".to_string(),
            ollama_model: "nomic-embed-text".to_string(),
            _model_path: None,
        }
    }

    /// Try to initialize Ollama embeddings
    pub async fn try_init_ollama(&self) -> bool {
        let client = reqwest::Client::new();
        let url = format!("{}/api/tags", self.ollama_url);

        match client.get(&url).timeout(std::time::Duration::from_secs(2)).send().await {
            Ok(resp) if resp.status().is_success() => {
                *self.provider.write() = EmbeddingProvider::Ollama;
                tracing::info!("Ollama embedding provider initialized");
                true
            }
            _ => {
                tracing::warn!("Ollama not available, using hash embeddings");
                false
            }
        }
    }

    /// Get current provider type
    pub fn provider(&self) -> EmbeddingProvider {
        self.provider.read().clone()
    }

    /// Get embedding dimension
    pub fn dimensions(&self) -> usize {
        EMBEDDING_DIM
    }

    /// Embed a single text
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>, String> {
        // Clone the provider to avoid holding lock across await
        let provider = self.provider.read().clone();
        match provider {
            EmbeddingProvider::Ollama => self.embed_ollama(text).await,
            EmbeddingProvider::Onnx => self.embed_onnx(text),
            EmbeddingProvider::Hash => Ok(self.embed_hash(text)),
        }
    }

    /// Embed multiple texts
    pub async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, String> {
        let mut results = Vec::with_capacity(texts.len());
        for text in texts {
            results.push(self.embed(text).await?);
        }
        Ok(results)
    }

    /// Ollama embedding via REST API
    async fn embed_ollama(&self, text: &str) -> Result<Vec<f32>, String> {
        let client = reqwest::Client::new();
        let url = format!("{}/api/embed", self.ollama_url);

        #[derive(Serialize)]
        struct EmbedRequest<'a> {
            model: &'a str,
            input: &'a str,
        }

        #[derive(Deserialize)]
        struct EmbedResponse {
            embeddings: Vec<Vec<f64>>,
        }

        let resp = client
            .post(&url)
            .json(&EmbedRequest {
                model: &self.ollama_model,
                input: text,
            })
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| format!("Ollama request failed: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!("Ollama returned status: {}", resp.status()));
        }

        let body: EmbedResponse = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse Ollama response: {}", e))?;

        if body.embeddings.is_empty() {
            return Err("No embeddings returned from Ollama".to_string());
        }

        let embedding: Vec<f32> = body.embeddings[0].iter().map(|&x| x as f32).collect();

        // Pad or truncate to EMBEDDING_DIM
        let mut result = if embedding.len() >= EMBEDDING_DIM {
            embedding[..EMBEDDING_DIM].to_vec()
        } else {
            let mut padded = embedding;
            padded.resize(EMBEDDING_DIM, 0.0);
            padded
        };

        normalize_vector(&mut result);
        Ok(result)
    }

    /// ONNX embedding (placeholder - loads model on demand)
    fn embed_onnx(&self, text: &str) -> Result<Vec<f32>, String> {
        // ONNX runtime integration is complex and requires model file.
        // For now, fall back to hash-based embedding with a warning.
        tracing::warn!("ONNX model not loaded, falling back to hash embeddings");
        Ok(self.embed_hash(text))
    }

    /// Hash-based embedding (deterministic, fast, but not semantic)
    /// Uses character n-gram hashing to produce a fixed-size vector.
    /// This provides basic word-level similarity but not true semantic understanding.
    fn embed_hash(&self, text: &str) -> Vec<f32> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut embedding = vec![0.0f32; EMBEDDING_DIM];
        let text_lower = text.to_lowercase();
        let words: Vec<&str> = text_lower.split_whitespace().collect();

        // Character trigram features
        let chars: Vec<char> = text_lower.chars().collect();
        for window in chars.windows(3) {
            let mut hasher = DefaultHasher::new();
            window.hash(&mut hasher);
            let idx = (hasher.finish() % EMBEDDING_DIM as u64) as usize;
            embedding[idx] += 1.0;
        }

        // Word-level features
        for word in &words {
            let mut hasher = DefaultHasher::new();
            word.hash(&mut hasher);
            let hash = hasher.finish();
            let idx1 = (hash % EMBEDDING_DIM as u64) as usize;
            let idx2 = ((hash >> 16) % EMBEDDING_DIM as u64) as usize;
            embedding[idx1] += 2.0;
            embedding[idx2] += 1.0;
        }

        // Word bigram features
        for pair in words.windows(2) {
            let mut hasher = DefaultHasher::new();
            pair[0].hash(&mut hasher);
            pair[1].hash(&mut hasher);
            let idx = (hasher.finish() % EMBEDDING_DIM as u64) as usize;
            embedding[idx] += 1.5;
        }

        normalize_vector(&mut embedding);
        embedding
    }
}

impl Default for EmbeddingModel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brain::utils::cosine_similarity;

    #[tokio::test]
    async fn test_hash_embedding_similarity() {
        let model = EmbeddingModel::new();

        let cat = model.embed("cat").await.unwrap();
        let _kitten = model.embed("kitten").await.unwrap();
        let car = model.embed("automobile racing championship").await.unwrap();

        // Hash embeddings won't be truly semantic, but identical strings should match
        let cat2 = model.embed("cat").await.unwrap();
        let self_sim = cosine_similarity(&cat, &cat2);
        assert!(
            (self_sim - 1.0).abs() < 1e-6,
            "Same text should have similarity 1.0"
        );

        // Different texts should have some difference
        let cat_car_sim = cosine_similarity(&cat, &car);
        assert!(
            cat_car_sim < 0.95,
            "Very different texts should have lower similarity"
        );
    }

    #[tokio::test]
    async fn test_embedding_dimensions() {
        let model = EmbeddingModel::new();
        let embedding = model.embed("test text").await.unwrap();
        assert_eq!(embedding.len(), EMBEDDING_DIM);
    }
}
