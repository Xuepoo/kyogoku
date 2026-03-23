use anyhow::Result;
use std::sync::{Arc, Mutex};
use tokio::sync::Semaphore;

use kyogoku_parser::TranslationBlock;

use crate::api::{ApiClient, ChatMessage};
use crate::cache::TranslationCache;
use crate::config::{Config, TranslationStyle};
use crate::glossary::Glossary;
use crate::rag::embeddings::EmbeddingModel;
use crate::rag::vectordb::VectorStore;

/// Translation engine that orchestrates the translation pipeline.
pub struct TranslationEngine {
    config: Config,
    client: ApiClient,
    cache: Option<TranslationCache>,
    glossary: Option<Glossary>,
    embedding_model: Option<Arc<EmbeddingModel>>,
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
            embedding_model: None,
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
    pub async fn translate_block(&self, block: &TranslationBlock) -> Result<String> {
        // Check cache first
        if let Some(ref cache) = self.cache
            && let Some(cached) = cache.get(&block.id)
        {
            tracing::debug!("Cache hit for block {}", block.id);
            return Ok(cached);
        }

        // Retrieve RAG context if enabled
        let rag_context = self.retrieve_rag_context(&block.source).await?;

        // Build prompt
        let prompt = self.build_prompt(block, &rag_context);

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
        
        // Update vector store (async background)
        if let Some(ref model) = self.embedding_model
            && let Some(ref store) = self.vector_store
        {
            let source = block.source.clone();
            let source_for_store = block.source.clone();
            let id = block.id.clone();
            let model = model.clone();
            let store = store.clone();
            
            tokio::spawn(async move {
                let embedding = tokio::task::spawn_blocking(move || {
                    model.embed(&[source])
                }).await;

                if let Ok(Ok(embeddings)) = embedding {
                    if let Some(vec) = embeddings.first() {
                         let mut store = store.lock().unwrap();
                         // Need to handle error properly in real app
                         if let Err(e) = store.add(id, vec.clone(), source_for_store) {
                             tracing::warn!("Failed to add to vector store: {}", e);
                         }
                    }
                }
            });
        }

        Ok(translation)
    }

    /// Translate multiple blocks with context window
    pub async fn translate_blocks<F>(
        &self,
        blocks: &mut [TranslationBlock],
        mut on_progress: F,
    ) -> Result<()> 
    where F: FnMut(usize, usize, &TranslationBlock)
    {
        let total = blocks.iter().filter(|b| b.needs_translation()).count();
        let mut completed = 0;

        // Collect previous translations for context
        let mut context_window: Vec<(String, String)> = Vec::new();
        let context_size = self.config.translation.context_size;

        for block in blocks.iter_mut() {
            if !block.needs_translation() {
                // Add to context if already translated
                if let Some(ref target) = block.target {
                    context_window.push((block.source.clone(), target.clone()));
                    if context_window.len() > context_size {
                        context_window.remove(0);
                    }
                }
                continue;
            }

            // Translate with context
            let translation = self
                .translate_block_with_context(block, &context_window)
                .await?;
            block.target = Some(translation.clone());

            // Update context window
            context_window.push((block.source.clone(), translation));
            if context_window.len() > context_size {
                context_window.remove(0);
            }

            completed += 1;
            on_progress(completed, total, block);
        }

        Ok(())
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
                let embedding = tokio::task::spawn_blocking(move || {
                    model.embed(&[source_for_embed])
                }).await;

                if let Ok(Ok(embeddings)) = embedding {
                    if let Some(vec) = embeddings.first() {
                         let mut store = store.lock().unwrap();
                         if let Err(e) = store.add(id, vec.clone(), source) {
                             tracing::warn!("Failed to add vector to store: {}", e);
                         } else {
                             // Auto-save occasionally? For now, maybe just keep in memory until explicit save?
                             // Or save on every add (slow).
                             // Ideally, save periodically.
                         }
                    }
                }
            });
        }

        Ok(translation)
    }

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
            let embedding = tokio::task::spawn_blocking(move || {
                model.embed(&[source])
            }).await??;

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
    async fn test_translate_blocks_progress_and_context_flow() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "chatcmpl-engine-2",
                "choices": [{
                    "message": {"role": "assistant", "content": "统一译文"},
                    "finish_reason": "stop"
                }],
                "usage": {"prompt_tokens": 20, "completion_tokens": 5, "total_tokens": 25}
            })))
            .expect(2)
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
        config.translation.context_size = 5;

        let engine = TranslationEngine::new(config).unwrap();
        let mut blocks = vec![
            TranslationBlock::new("第一句"),
            TranslationBlock::new("第二句"),
        ];
        let mut progress_calls = Vec::new();

        engine
            .translate_blocks(&mut blocks, |done, total, _| {
                progress_calls.push((done, total))
            })
            .await
            .unwrap();

        assert_eq!(blocks[0].target.as_deref(), Some("统一译文"));
        assert_eq!(blocks[1].target.as_deref(), Some("统一译文"));
        assert_eq!(progress_calls, vec![(1, 2), (2, 2)]);
    }
}
