//! File indexer for SuperBrain
//!
//! Watches filesystem, chunks files, and indexes them with vector embeddings
//! for semantic file search.

pub mod chunker;
pub mod parser;
pub mod watcher;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use parking_lot::RwLock;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::brain::embeddings::EmbeddingModel;
use crate::brain::utils::cosine_similarity;

/// File search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileResult {
    pub path: String,
    pub name: String,
    pub chunk: String,
    pub similarity: f64,
    pub file_type: String,
}

/// File index entry stored in SQLite
#[derive(Debug, Clone)]
struct FileEntry {
    path: String,
    name: String,
    ext: String,
    modified: i64,
    chunk_count: u32,
}

/// File chunk with embedding
#[derive(Debug, Clone)]
struct FileChunk {
    file_path: String,
    chunk_index: u32,
    content: String,
    vector: Vec<f32>,
}

/// The file indexer manages scanning, watching, and searching files
pub struct FileIndexer {
    db_path: PathBuf,
    watched_dirs: RwLock<Vec<PathBuf>>,
    embeddings: Arc<EmbeddingModel>,
    is_indexing: RwLock<bool>,
}

impl FileIndexer {
    /// Create a new file indexer
    pub fn new(db_path: PathBuf, embeddings: Arc<EmbeddingModel>) -> Result<Self, String> {
        let indexer = Self {
            db_path,
            watched_dirs: RwLock::new(Vec::new()),
            embeddings,
            is_indexing: RwLock::new(false),
        };
        indexer.initialize_db()?;
        Ok(indexer)
    }

    fn open_connection(&self) -> Result<Connection, String> {
        Connection::open(&self.db_path).map_err(|e| format!("DB open failed: {}", e))
    }

    fn initialize_db(&self) -> Result<(), String> {
        let conn = self.open_connection()?;
        conn.execute_batch(
            "
            PRAGMA journal_mode=WAL;

            CREATE TABLE IF NOT EXISTS file_index (
                path TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                ext TEXT NOT NULL,
                modified INTEGER NOT NULL,
                chunk_count INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS file_chunks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                file_path TEXT NOT NULL,
                chunk_index INTEGER NOT NULL,
                content TEXT NOT NULL,
                vector BLOB NOT NULL,
                FOREIGN KEY (file_path) REFERENCES file_index(path) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_chunks_path ON file_chunks(file_path);
            ",
        )
        .map_err(|e| format!("DB init failed: {}", e))?;
        Ok(())
    }

    /// Add directories to watch
    pub fn add_watch_dirs(&self, dirs: Vec<PathBuf>) {
        let mut watched = self.watched_dirs.write();
        for dir in dirs {
            if dir.exists() && !watched.contains(&dir) {
                watched.push(dir);
            }
        }
    }

    /// Index a single file
    pub async fn index_file(&self, path: &Path) -> Result<u32, String> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        if !parser::is_supported(&ext) {
            return Ok(0);
        }

        let content = parser::parse_file(path)?;
        if content.trim().is_empty() {
            return Ok(0);
        }

        let chunks = chunker::chunk_text(&content, 512, 128);
        if chunks.is_empty() {
            return Ok(0);
        }

        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let modified = path
            .metadata()
            .map(|m| {
                m.modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0)
            })
            .unwrap_or(0);

        let path_str = path.to_string_lossy().to_string();

        // Embed all chunks
        let mut file_chunks = Vec::with_capacity(chunks.len());
        for (i, chunk) in chunks.iter().enumerate() {
            let vector = self.embeddings.embed(chunk).await?;
            file_chunks.push(FileChunk {
                file_path: path_str.clone(),
                chunk_index: i as u32,
                content: chunk.clone(),
                vector,
            });
        }

        // Store in database
        let conn = self.open_connection()?;

        // Upsert file entry
        conn.execute(
            "INSERT OR REPLACE INTO file_index (path, name, ext, modified, chunk_count) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![path_str, name, ext, modified, chunks.len() as u32],
        )
        .map_err(|e| format!("Store file failed: {}", e))?;

        // Delete old chunks
        conn.execute(
            "DELETE FROM file_chunks WHERE file_path = ?1",
            params![path_str],
        )
        .map_err(|e| format!("Delete chunks failed: {}", e))?;

        // Insert new chunks
        for chunk in &file_chunks {
            let vector_bytes = vector_to_bytes(&chunk.vector);
            conn.execute(
                "INSERT INTO file_chunks (file_path, chunk_index, content, vector) VALUES (?1, ?2, ?3, ?4)",
                params![chunk.file_path, chunk.chunk_index, chunk.content, vector_bytes],
            )
            .map_err(|e| format!("Store chunk failed: {}", e))?;
        }

        Ok(file_chunks.len() as u32)
    }

    /// Scan and index all files in watched directories (recursive)
    pub async fn scan_all(&self) -> Result<u32, String> {
        {
            let is_indexing = self.is_indexing.read();
            if *is_indexing {
                return Err("Indexing already in progress".to_string());
            }
        }
        *self.is_indexing.write() = true;

        let dirs: Vec<PathBuf> = self.watched_dirs.read().clone();
        let mut total = 0u32;

        // Collect all files recursively first
        let mut files = Vec::new();
        for dir in &dirs {
            collect_files_recursive(dir, &mut files, 10);
        }

        tracing::info!("Found {} files to index", files.len());

        for path in &files {
            match self.index_file(path).await {
                Ok(chunks) => total += chunks,
                Err(e) => tracing::debug!("Skipped {:?}: {}", path, e),
            }
        }

        *self.is_indexing.write() = false;
        tracing::info!("Indexed {} chunks from {} files", total, files.len());
        Ok(total)
    }

    /// Search indexed files by semantic similarity
    pub async fn search(&self, query: &str, limit: u32) -> Result<Vec<FileResult>, String> {
        let query_vector = self.embeddings.embed(query).await?;

        let conn = self.open_connection()?;
        let mut stmt = conn
            .prepare(
                "SELECT fc.file_path, fc.content, fc.vector, fi.name, fi.ext
                 FROM file_chunks fc
                 JOIN file_index fi ON fc.file_path = fi.path",
            )
            .map_err(|e| format!("Query failed: {}", e))?;

        let mut results: Vec<FileResult> = stmt
            .query_map([], |row| {
                let file_path: String = row.get(0)?;
                let content: String = row.get(1)?;
                let vector_bytes: Vec<u8> = row.get(2)?;
                let name: String = row.get(3)?;
                let ext: String = row.get(4)?;

                let vector = bytes_to_vector(&vector_bytes);
                let similarity = cosine_similarity(&query_vector, &vector) as f64;

                Ok(FileResult {
                    path: file_path,
                    name,
                    chunk: content,
                    similarity,
                    file_type: ext,
                })
            })
            .map_err(|e| format!("Search failed: {}", e))?
            .filter_map(|r| r.ok())
            .filter(|r| r.similarity > 0.1)
            .collect();

        results.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit as usize);

        Ok(results)
    }

    /// Get index statistics
    pub fn stats(&self) -> Result<IndexStats, String> {
        let conn = self.open_connection()?;

        let file_count: u32 = conn
            .query_row("SELECT COUNT(*) FROM file_index", [], |row| row.get(0))
            .unwrap_or(0);

        let chunk_count: u32 = conn
            .query_row("SELECT COUNT(*) FROM file_chunks", [], |row| row.get(0))
            .unwrap_or(0);

        let is_indexing = *self.is_indexing.read();

        Ok(IndexStats {
            file_count,
            chunk_count,
            watched_dirs: self.watched_dirs.read().len() as u32,
            is_indexing,
        })
    }
}

/// Index statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStats {
    pub file_count: u32,
    pub chunk_count: u32,
    pub watched_dirs: u32,
    pub is_indexing: bool,
}

fn vector_to_bytes(vector: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(vector.len() * 4);
    for &val in vector {
        bytes.extend_from_slice(&val.to_le_bytes());
    }
    bytes
}

fn bytes_to_vector(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}

/// Directories to skip during recursive scanning
const SKIP_DIRS: &[&str] = &[
    "node_modules",
    "target",
    ".git",
    ".svn",
    ".hg",
    "__pycache__",
    ".venv",
    "venv",
    ".cache",
    "build",
    "dist",
    ".Trash",
    "Library",
];

/// Recursively collect files, skipping hidden/undesirable directories
fn collect_files_recursive(dir: &Path, files: &mut Vec<PathBuf>, max_depth: u32) {
    if max_depth == 0 {
        return;
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // Skip hidden files/dirs
        if name_str.starts_with('.') {
            continue;
        }

        if path.is_dir() {
            // Skip known undesirable directories
            if SKIP_DIRS.contains(&name_str.as_ref()) {
                continue;
            }
            collect_files_recursive(&path, files, max_depth - 1);
        } else if path.is_file() {
            files.push(path);
        }
    }
}
