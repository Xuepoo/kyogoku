use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};

pub trait VectorStore {
    fn add(&mut self, id: String, embedding: Vec<f32>, source: String) -> Result<()>;
    fn search(&self, query: &[f32], limit: usize) -> Result<Vec<(String, f32, String)>>;
    fn save(&self) -> Result<()>;
    fn load(&mut self) -> Result<()>;
}

#[derive(Serialize, Deserialize, Default)]
pub struct SimpleVectorStore {
    path: PathBuf,
    vectors: HashMap<String, (Vec<f32>, String)>, // (embedding, source_text)
}

impl SimpleVectorStore {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            vectors: HashMap::new(),
        }
    }

    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        let dot_product: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm_a == 0.0 || norm_b == 0.0 {
            0.0
        } else {
            dot_product / (norm_a * norm_b)
        }
    }
}

impl VectorStore for SimpleVectorStore {
    fn add(&mut self, id: String, embedding: Vec<f32>, source: String) -> Result<()> {
        self.vectors.insert(id, (embedding, source));
        Ok(())
    }

    fn search(&self, query: &[f32], limit: usize) -> Result<Vec<(String, f32, String)>> {
        let mut scores: Vec<(String, f32, String)> = self
            .vectors
            .iter()
            .map(|(id, (vec, source))| {
                (
                    id.clone(),
                    Self::cosine_similarity(query, vec),
                    source.clone(),
                )
            })
            .collect();

        // Sort by score descending
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(limit);

        Ok(scores)
    }

    fn save(&self) -> Result<()> {
        let file = File::create(&self.path).context("Failed to create vector store file")?;
        let writer = BufWriter::new(file);
        bincode::serialize_into(writer, &self.vectors)
            .context("Failed to serialize vector store")?;
        Ok(())
    }

    fn load(&mut self) -> Result<()> {
        if !self.path.exists() {
            return Ok(());
        }
        let file = File::open(&self.path).context("Failed to open vector store file")?;
        let reader = BufReader::new(file);
        self.vectors =
            bincode::deserialize_from(reader).context("Failed to deserialize vector store")?;
        Ok(())
    }
}
