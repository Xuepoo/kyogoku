use anyhow::Result;
use ndarray::{Array2, ArrayView};
use ort::{session::Session, session::builder::GraphOptimizationLevel, value::Value};
use std::path::Path;
use tokenizers::Tokenizer;

use std::sync::Mutex;

pub struct EmbeddingModel {
    tokenizer: Tokenizer,
    session: Mutex<Session>,
}

impl EmbeddingModel {
    pub fn new<P: AsRef<Path>>(model_path: P, tokenizer_path: P) -> Result<Self> {
        let tokenizer = Tokenizer::from_file(tokenizer_path).map_err(|e| anyhow::anyhow!(e))?;

        let session = Session::builder()
            .map_err(|e| anyhow::anyhow!("{}", e))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| anyhow::anyhow!("{}", e))?
            .with_intra_threads(4)
            .map_err(|e| anyhow::anyhow!("{}", e))?
            .commit_from_file(model_path)
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        Ok(Self {
            tokenizer,
            session: Mutex::new(session),
        })
    }

    pub fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let encoding = self
            .tokenizer
            .encode_batch(texts.to_vec(), true)
            .map_err(|e| anyhow::anyhow!(e))?;

        let batch_size = texts.len();
        let max_len = encoding
            .iter()
            .map(|e| e.get_ids().len())
            .max()
            .unwrap_or(0);

        let mut input_ids = Array2::<i64>::zeros((batch_size, max_len));
        let mut attention_mask = Array2::<i64>::zeros((batch_size, max_len));
        let mut token_type_ids = Array2::<i64>::zeros((batch_size, max_len));

        for (i, encode) in encoding.iter().enumerate() {
            let ids = encode.get_ids();
            let mask = encode.get_attention_mask();
            let type_ids = encode.get_type_ids();

            for (j, &id) in ids.iter().enumerate() {
                input_ids[[i, j]] = id as i64;
                attention_mask[[i, j]] = mask[j] as i64;
                token_type_ids[[i, j]] = type_ids[j] as i64;
            }
        }

        // Clone mask because we need it for pooling later
        let mask_for_pooling = attention_mask.clone();

        let v_ids = Value::from_array(input_ids)?;
        let v_mask = Value::from_array(attention_mask)?;
        let v_type = Value::from_array(token_type_ids)?;

        let inputs = ort::inputs![
            "input_ids" => v_ids,
            "attention_mask" => v_mask,
            "token_type_ids" => v_type
        ];

        let mut session = self
            .session
            .lock()
            .map_err(|e| anyhow::anyhow!("Session lock poisoned: {}", e))?;
        let outputs = session.run(inputs)?;
        let (shape, data) = outputs["last_hidden_state"].try_extract_tensor::<f32>()?;

        // Mean pooling: sum(last_hidden_state * attention_mask) / sum(attention_mask)

        // shape is likely [batch, seq_len, hidden_size]
        let shape_vec: Vec<usize> = shape.iter().map(|&x| x as usize).collect();
        let batch_embeddings = ArrayView::from_shape(shape_vec, data)?;
        let hidden_size = batch_embeddings.shape()[2];

        let mut embeddings = Vec::with_capacity(batch_size);

        for i in 0..batch_size {
            let mut sum_vec = vec![0.0; hidden_size];
            let mut count = 0.0;

            for j in 0..max_len {
                if mask_for_pooling[[i, j]] == 1 {
                    for k in 0..hidden_size {
                        sum_vec[k] += batch_embeddings[[i, j, k]];
                    }
                    count += 1.0;
                }
            }

            if count > 0.0 {
                for val in sum_vec.iter_mut() {
                    *val /= count;
                }
            }

            // Normalize
            let norm = sum_vec.iter().map(|x| x * x).sum::<f32>().sqrt();
            if norm > 0.0 {
                for val in sum_vec.iter_mut() {
                    *val /= norm;
                }
            }

            embeddings.push(sum_vec);
        }

        Ok(embeddings)
    }
}
