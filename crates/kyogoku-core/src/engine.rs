use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Semaphore;

use kyogoku_parser::TranslationBlock;

use crate::api::{ApiClient, ChatMessage};
use crate::cache::TranslationCache;
use crate::config::{Config, TranslationStyle};
use crate::glossary::Glossary;

/// Translation engine that orchestrates the translation pipeline.
pub struct TranslationEngine {
    config: Config,
    client: ApiClient,
    cache: Option<TranslationCache>,
    glossary: Option<Glossary>,
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

    /// Translate a single block
    pub async fn translate_block(&self, block: &TranslationBlock) -> Result<String> {
        // Check cache first
        if let Some(ref cache) = self.cache
            && let Some(cached) = cache.get(&block.id)
        {
            tracing::debug!("Cache hit for block {}", block.id);
            return Ok(cached);
        }

        // Build prompt
        let prompt = self.build_prompt(block);

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

        Ok(translation)
    }

    /// Translate multiple blocks with context window
    pub async fn translate_blocks(
        &self,
        blocks: &mut [TranslationBlock],
        on_progress: impl Fn(usize, usize),
    ) -> Result<()> {
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
            on_progress(completed, total);
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

        // Build prompt with context
        let prompt = self.build_prompt_with_context(block, context);

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

        Ok(translation)
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

    fn build_prompt(&self, block: &TranslationBlock) -> String {
        let mut prompt = String::new();

        // Add glossary if available
        if let Some(ref glossary) = self.glossary
            && let Some(glossary_text) = glossary.format_for_prompt(&block.source)
        {
            prompt.push_str(&glossary_text);
            prompt.push_str("\n\n");
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
    ) -> String {
        let mut prompt = String::new();

        // Add glossary if available
        if let Some(ref glossary) = self.glossary
            && let Some(glossary_text) = glossary.format_for_prompt(&block.source)
        {
            prompt.push_str(&glossary_text);
            prompt.push_str("\n\n");
        }

        // Add context window
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

    #[test]
    fn test_engine_creation() {
        let config = Config::default();
        let engine = TranslationEngine::new(config);
        assert!(engine.is_ok());
    }
}
