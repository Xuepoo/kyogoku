use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{debug, info, instrument};

#[cfg(feature = "rag")]
use std::sync::Mutex;

use kyogoku_parser::TranslationBlock;

use crate::api::{ApiClient, ChatMessage};
use crate::cache::TranslationCache;
use crate::config::{Config, TranslationStyle};
use crate::glossary::Glossary;

#[cfg(feature = "rag")]
use crate::rag::embeddings::EmbeddingModel;
#[cfg(feature = "rag")]
use crate::rag::vectordb::VectorStore;

/// Translation engine that orchestrates the translation pipeline.
pub struct TranslationEngine {
    config: Config,
    client: ApiClient,
    cache: Option<TranslationCache>,
    glossary: Option<Glossary>,
    #[cfg(feature = "rag")]
    embedding_model: Option<Arc<EmbeddingModel>>,
    #[cfg(feature = "rag")]
    vector_store: Option<Arc<Mutex<dyn VectorStore + Send + Sync>>>,
    semaphore: Arc<Semaphore>,
}

impl TranslationEngine {
    pub fn new(config: Config) -> Result<Self> {
        let client = ApiClient::new(config.api.clone())?;
        let semaphore = Arc::new(Semaphore::new(config.advanced.max_concurrency));

        Ok(Self {
            config,
            client,
            cache: None,
            glossary: None,
            #[cfg(feature = "rag")]
            embedding_model: None,
            #[cfg(feature = "rag")]
            vector_store: None,
            semaphore,
        })
    }

    pub fn with_cache(mut self, cache: TranslationCache) -> Self {
        self.cache = Some(cache);
        self
    }

    pub fn with_glossary(mut self, glossary: Glossary) -> Self {
        self.glossary = Some(glossary);
        self
    }

    #[cfg(feature = "rag")]
    pub fn with_rag(
        mut self,
        embedding_model: Arc<EmbeddingModel>,
        vector_store: Arc<Mutex<dyn VectorStore + Send + Sync>>,
    ) -> Self {
        self.embedding_model = Some(embedding_model);
        self.vector_store = Some(vector_store);
        self
    }

    /// Translate a single block
    #[instrument(skip(self, block), fields(block_id = %block.id, source_len = block.source.len()))]
    pub async fn translate_block(&self, block: &TranslationBlock) -> Result<String> {
        // Check cache first
        if let Some(ref cache) = self.cache
            && let Some(cached) = cache.get(&block.id)
        {
            debug!("Cache hit");
            return Ok(cached);
        }

        // Retrieve RAG context if enabled
        let rag_context = self.retrieve_rag_context(&block.source).await?;

        // Build prompt
        let prompt = self.build_prompt(block, &rag_context);

        // Call API
        let _permit = self.semaphore.acquire().await.unwrap();
        debug!("Acquired semaphore permit, calling API");
        let translation = self
            .client
            .chat(vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: self.system_prompt(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: prompt,
                },
            ])
            .await?;

        // Store in cache
        if let Some(ref cache) = self.cache {
            cache.set(&block.id, &translation)?;
            debug!("Stored in cache");
        }

        // Update vector store (async background)
        #[cfg(feature = "rag")]
        if let Some(ref model) = self.embedding_model
            && let Some(ref store) = self.vector_store
        {
            let source = block.source.clone();
            let source_for_store = block.source.clone();
            let id = block.id.clone();
            let model = model.clone();
            let store = store.clone();

            tokio::spawn(async move {
                let embedding = tokio::task::spawn_blocking(move || model.embed(&[source])).await;

                if let Ok(Ok(embeddings)) = embedding
                    && let Some(vec) = embeddings.first()
                {
                    let mut store = store.lock().unwrap();
                    if let Err(e) = store.add(id, vec.clone(), source_for_store) {
                        tracing::warn!("Failed to add to vector store: {}", e);
                    }
                }
            });
        }

        Ok(translation)
    }

    /// Translate multiple blocks with context window
    #[instrument(skip(self, blocks, on_progress), fields(total_blocks = blocks.len()))]
    pub async fn translate_blocks<F>(
        &self,
        blocks: &mut [TranslationBlock],
        mut on_progress: F,
    ) -> Result<()>
    where
        F: FnMut(usize, usize, &TranslationBlock),
    {
        let total = blocks.iter().filter(|b| b.needs_translation()).count();
        info!(needs_translation = total, "Starting batch translation");
        let mut completed = 0;

        // Collect previous translations for context
        let mut context_window: Vec<(String, String)> = Vec::new();
        let context_size = self.config.translation.context_size;
        let batch_size = self.config.advanced.batch_size;

        let mut batch = Vec::with_capacity(batch_size);

        for block in blocks.iter_mut() {
            // Check if block needs translation
            let needs_translation = block.needs_translation();

            // Try cache if needed
            let mut cached = false;
            if needs_translation
                && let Some(ref cache) = self.cache
                && let Some(target) = cache.get(&block.id)
            {
                block.target = Some(target);
                cached = true;
                completed += 1;
                on_progress(completed, total, block);
            }

            if !needs_translation || cached {
                // If we have a pending batch, process it first because this block
                // might depend on previous ones, or future ones depend on this.
                // Actually, if this block is done, we can use it as context for the batch?
                // No, the batch was accumulated *before* this block, so this block is *future* for them.
                // So we can process the batch using current `context_window`.
                if !batch.is_empty() {
                    self.process_batch(&mut batch, &mut context_window).await?;
                    for b in batch.drain(..) {
                        completed += 1;
                        on_progress(completed, total, b);
                    }
                }

                // Add this block to context
                if let Some(ref target) = block.target {
                    context_window.push((block.source.clone(), target.clone()));
                    if context_window.len() > context_size {
                        context_window.remove(0);
                    }
                }
                continue;
            }

            // Add to batch
            batch.push(block);

            // Process if full
            if batch.len() >= batch_size {
                self.process_batch(&mut batch, &mut context_window).await?;
                for b in batch.drain(..) {
                    completed += 1;
                    on_progress(completed, total, b);
                }
            }
        }

        // Process remaining
        if !batch.is_empty() {
            self.process_batch(&mut batch, &mut context_window).await?;
            for b in batch.drain(..) {
                completed += 1;
                on_progress(completed, total, b);
            }
        }

        Ok(())
    }

    async fn process_batch(
        &self,
        batch: &mut Vec<&mut TranslationBlock>,
        context: &mut Vec<(String, String)>,
    ) -> Result<()> {
        tracing::debug!("Processing batch of {} blocks", batch.len());
        let translations = self.translate_batch_with_context(batch, context).await?;

        for (i, translation) in translations.into_iter().enumerate() {
            if i < batch.len() {
                let block = &mut batch[i];
                block.target = Some(translation.clone());

                // Update context
                context.push((block.source.clone(), translation.clone()));
                if context.len() > self.config.translation.context_size {
                    context.remove(0);
                }

                // Update cache
                if let Some(ref cache) = self.cache {
                    let _ = cache.set(&block.id, &translation);
                }

                // Update vector store (skip for now to keep simple, or implement batch embedding)
                // Doing it sequentially here is fine for now
            }
        }

        Ok(())
    }

    async fn translate_batch_with_context(
        &self,
        blocks: &[&mut TranslationBlock],
        context: &[(String, String)],
    ) -> Result<Vec<String>> {
        // Optimization: If only 1 block, use standard single-block translation
        // This avoids confusion with separators for the model
        if blocks.len() == 1 {
            let translation = self
                .translate_block_with_context(blocks[0], context)
                .await?;
            return Ok(vec![translation]);
        }

        // Collect RAG context for the first block (or all?)
        // For simplicity, use the first block's RAG context for the batch
        let rag_context = if let Some(first) = blocks.first() {
            self.retrieve_rag_context(&first.source).await?
        } else {
            Vec::new()
        };

        // Build batch prompt
        let prompt = self.build_batch_prompt(blocks, context, &rag_context);

        // Call API
        let _permit = self.semaphore.acquire().await.unwrap();
        let response = self
            .client
            .chat(vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: self.system_prompt(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: prompt,
                },
            ])
            .await?;

        // Parse response
        let separator = "<<<SEPARATOR>>>";
        let parts: Vec<String> = response
            .split(separator)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()) // Filter empty parts (handles trailing separators)
            .collect();

        // If count mismatch, fallback to individual translation (or just error/warn)
        if parts.len() != blocks.len() {
            tracing::warn!(
                "Batch translation mismatch: expected {}, got {}. Fallback to individual.",
                blocks.len(),
                parts.len()
            );

            // Fallback: translate one by one
            let mut results = Vec::new();
            // Note: This is recursive but with batch size 1 effectively
            // We can't easily call translate_block_with_context because we are inside the method.
            // But we can just loop here.
            for block in blocks {
                // We need to rebuild context for each if we want to be precise,
                // but here we just reuse the initial context for simplicity
                // (or we can't because we are in a &self method, not modifying context).
                // Actually, translate_block_with_context does not modify context.
                let t = self.translate_block_with_context(block, context).await?;
                results.push(t);
            }
            return Ok(results);
        }

        Ok(parts)
    }

    fn build_batch_prompt(
        &self,
        blocks: &[&mut TranslationBlock],
        context: &[(String, String)],
        rag_context: &[(String, String)],
    ) -> String {
        let mut prompt = String::new();

        // Glossary (use first block's source for glossary matching, or combine?)
        // Better: combine all sources
        let combined_source: String = blocks
            .iter()
            .map(|b| b.source.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        if let Some(ref glossary) = self.glossary
            && let Some(glossary_text) = glossary.format_for_prompt(&combined_source)
        {
            prompt.push_str(&glossary_text);
            prompt.push_str("\n\n");
        }

        // RAG Context
        if !rag_context.is_empty() {
            prompt.push_str("参考译文（来自类似文本）：\n");
            for (src, tgt) in rag_context {
                prompt.push_str(&format!("参考原文: {}\n参考译文: {}\n\n", src, tgt));
            }
            prompt.push('\n');
        }

        // Context Window
        if !context.is_empty() {
            prompt.push_str("前文参考：\n");
            for (src, tgt) in context.iter().rev().take(5) {
                prompt.push_str(&format!("原文: {}\n译文: {}\n\n", src, tgt));
            }
            prompt.push('\n');
        }

        prompt.push_str("请按顺序翻译以下文本片段，每个结果之间必须用 <<<SEPARATOR>>> 分隔（不要包含原文）：\n\n");

        for (i, block) in blocks.iter().enumerate() {
            if let Some(ref speaker) = block.speaker {
                prompt.push_str(&format!("[片段{}] (说话人: {})\n", i + 1, speaker));
            } else {
                prompt.push_str(&format!("[片段{}]\n", i + 1));
            }
            prompt.push_str(&block.source);
            prompt.push_str("\n\n");
        }

        prompt
    }

    async fn translate_block_with_context(
        &self,
        block: &TranslationBlock,
        context: &[(String, String)],
    ) -> Result<String> {
        // Check cache first
        if let Some(ref cache) = self.cache
            && let Some(cached) = cache.get(&block.id)
        {
            tracing::debug!("Cache hit for block {}", block.id);
            return Ok(cached);
        }

        // Retrieve RAG context if enabled
        let rag_context = self.retrieve_rag_context(&block.source).await?;

        // Build prompt with context
        let prompt = self.build_prompt_with_context(block, context, &rag_context);

        // Call API
        let _permit = self.semaphore.acquire().await.unwrap();
        let translation = self
            .client
            .chat(vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: self.system_prompt(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: prompt,
                },
            ])
            .await?;

        // Store in cache
        if let Some(ref cache) = self.cache {
            cache.set(&block.id, &translation)?;
        }

        // Update vector store
        #[cfg(feature = "rag")]
        if let Some(ref model) = self.embedding_model
            && let Some(ref store) = self.vector_store
        {
            let source = block.source.clone();
            let id = block.id.clone();
            let model = model.clone();
            let store = store.clone();

            // Run embedding in background to avoid blocking translation flow too much
            tokio::spawn(async move {
                // Compute embedding
                // Note: embed is sync/blocking, so use spawn_blocking
                let source_for_embed = source.clone();
                let embedding =
                    tokio::task::spawn_blocking(move || model.embed(&[source_for_embed])).await;

                if let Ok(Ok(embeddings)) = embedding
                    && let Some(vec) = embeddings.first()
                {
                    let mut store = store.lock().unwrap();
                    if let Err(e) = store.add(id, vec.clone(), source) {
                        tracing::warn!("Failed to add vector to store: {}", e);
                    } else {
                        // Auto-save occasionally? For now, maybe just keep in memory until explicit save?
                        // Or save on every add (slow).
                        // Ideally, save periodically.
                    }
                }
            });
        }

        Ok(translation)
    }

    #[cfg(feature = "rag")]
    async fn retrieve_rag_context(&self, source: &str) -> Result<Vec<(String, String)>> {
        let mut context = Vec::new();

        if let Some(ref model) = self.embedding_model
            && let Some(ref store) = self.vector_store
            && let Some(ref cache) = self.cache
        {
            let source = source.to_string();
            let model = model.clone();
            let store = store.clone();

            // Compute embedding
            let embedding = tokio::task::spawn_blocking(move || model.embed(&[source])).await??;

            if let Some(query_vec) = embedding.first() {
                // Search vector store
                let results = {
                    let store = store.lock().unwrap();
                    store.search(query_vec, 3)? // Top 3 similar
                };

                for (id, _score, source_text) in results {
                    if let Some(target) = cache.get(&id) {
                        context.push((source_text, target));
                    }
                }
            }
        }

        Ok(context)
    }

    #[cfg(not(feature = "rag"))]
    async fn retrieve_rag_context(&self, _source: &str) -> Result<Vec<(String, String)>> {
        Ok(Vec::new())
    }

    fn system_prompt(&self) -> String {
        let style = match self.config.translation.style {
            TranslationStyle::Literary => "文学风格，保持原文的修辞和美感",
            TranslationStyle::Casual => "口语化风格，自然流畅",
            TranslationStyle::Formal => "正式风格，用词严谨",
            TranslationStyle::Technical => "技术文档风格，准确专业",
        };

        format!(
            r#"你是一位专业的翻译专家，擅长将{}翻译成{}。

翻译要求：
- {}
- 保持原文的语气和风格
- 保留所有特殊标记和控制符
- 人名、地名等专有名词参考术语表
- 只输出译文，不要添加任何解释"#,
            self.config.project.source_lang, self.config.project.target_lang, style
        )
    }

    fn build_prompt(&self, block: &TranslationBlock, rag_context: &[(String, String)]) -> String {
        let mut prompt = String::new();

        // Add glossary if available
        if let Some(ref glossary) = self.glossary
            && let Some(glossary_text) = glossary.format_for_prompt(&block.source)
        {
            prompt.push_str(&glossary_text);
            prompt.push_str("\n\n");
        }

        // Add RAG context
        if !rag_context.is_empty() {
            prompt.push_str("参考译文：\n");
            for (src, tgt) in rag_context {
                prompt.push_str(&format!("原文: {}\n译文: {}\n\n", src, tgt));
            }
            prompt.push('\n');
        }

        // Add speaker context if available
        if let Some(ref speaker) = block.speaker {
            prompt.push_str(&format!("说话人: {}\n\n", speaker));
        }

        prompt.push_str(&format!("请翻译以下文本：\n{}", block.source));

        prompt
    }

    fn build_prompt_with_context(
        &self,
        block: &TranslationBlock,
        context: &[(String, String)],
        rag_context: &[(String, String)],
    ) -> String {
        let mut prompt = String::new();

        // Add glossary if available
        if let Some(ref glossary) = self.glossary
            && let Some(glossary_text) = glossary.format_for_prompt(&block.source)
        {
            prompt.push_str(&glossary_text);
            prompt.push_str("\n\n");
        }

        // Add RAG context (Reference Translations)
        if !rag_context.is_empty() {
            prompt.push_str("参考译文（来自类似文本）：\n");
            for (src, tgt) in rag_context {
                prompt.push_str(&format!("参考原文: {}\n参考译文: {}\n\n", src, tgt));
            }
            prompt.push('\n');
        }

        // Add context window (Immediate Context)
        if !context.is_empty() {
            prompt.push_str("前文参考：\n");
            for (src, tgt) in context.iter().rev().take(5) {
                prompt.push_str(&format!("原文: {}\n译文: {}\n\n", src, tgt));
            }
            prompt.push('\n');
        }

        // Add speaker context if available
        if let Some(ref speaker) = block.speaker {
            prompt.push_str(&format!("说话人: {}\n\n", speaker));
        }

        prompt.push_str(&format!("请翻译以下文本：\n{}", block.source));

        prompt
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::TranslationCache;
    use crate::config::{ApiConfig, ApiProvider};
    use tempfile::TempDir;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn test_engine_creation() {
        let config = Config::default();
        let engine = TranslationEngine::new(config);
        assert!(engine.is_ok());
    }

    #[tokio::test]
    async fn test_translate_block_uses_mock_api_and_cache() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "chatcmpl-engine",
                "choices": [{
                    "message": {"role": "assistant", "content": "翻译结果"},
                    "finish_reason": "stop"
                }],
                "usage": {"prompt_tokens": 8, "completion_tokens": 4, "total_tokens": 12}
            })))
            .expect(1)
            .mount(&server)
            .await;

        let config = Config {
            api: ApiConfig {
                provider: ApiProvider::Custom,
                api_base: Some(format!("{}/v1", server.uri())),
                api_key: Some("test-key".to_string()),
                model: "mock-model".to_string(),
                ..ApiConfig::default()
            },
            ..Config::default()
        };

        let tmp = TempDir::new().unwrap();
        let cache = TranslationCache::open(tmp.path()).unwrap();
        let engine = TranslationEngine::new(config).unwrap().with_cache(cache);

        let block = TranslationBlock::new("原文测试");
        let first = engine.translate_block(&block).await.unwrap();
        assert_eq!(first, "翻译结果");

        let second = engine.translate_block(&block).await.unwrap();
        assert_eq!(second, "翻译结果");
    }

    #[tokio::test]
    async fn test_translate_blocks_batching() {
        let server = MockServer::start().await;

        // Expect 2 requests:
        // 1. Batch of 2 blocks
        // 2. Batch of 1 block
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "chatcmpl-batch",
                "choices": [{
                    "message": {"role": "assistant", "content": "译文1<<<SEPARATOR>>>译文2"},
                    "finish_reason": "stop"
                }],
                "usage": {"prompt_tokens": 20, "completion_tokens": 10, "total_tokens": 30}
            })))
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "chatcmpl-batch-2",
                "choices": [{
                    "message": {"role": "assistant", "content": "译文3"},
                    "finish_reason": "stop"
                }],
                "usage": {"prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15}
            })))
            .mount(&server)
            .await;

        let mut config = Config {
            api: ApiConfig {
                provider: ApiProvider::Custom,
                api_base: Some(format!("{}/v1", server.uri())),
                api_key: Some("test-key".to_string()),
                model: "mock-model".to_string(),
                ..ApiConfig::default()
            },
            ..Config::default()
        };
        config.advanced.batch_size = 2; // Set batch size to 2

        let engine = TranslationEngine::new(config).unwrap();
        let mut blocks = vec![
            TranslationBlock::new("原文1"),
            TranslationBlock::new("原文2"),
            TranslationBlock::new("原文3"),
        ];

        engine
            .translate_blocks(&mut blocks, |_, _, _| {})
            .await
            .unwrap();

        assert_eq!(blocks[0].target.as_deref(), Some("译文1"));
        assert_eq!(blocks[1].target.as_deref(), Some("译文2"));
        assert_eq!(blocks[2].target.as_deref(), Some("译文3"));
    }
}
