use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, instrument, warn};

use crate::config::{ApiConfig, ApiProvider};

#[derive(Debug, Clone, Serialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChatChoice {
    pub message: ChatMessageResponse,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChatMessageResponse {
    pub role: String,
    pub content: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChatResponse {
    pub id: String,
    pub choices: Vec<ChatChoice>,
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

pub struct ApiClient {
    client: reqwest::Client,
    config: ApiConfig,
    max_retries: u32,
}

impl ApiClient {
    pub fn new(mut config: ApiConfig) -> Result<Self> {
        // Resolve API key from environment if needed
        config.api_key = config.resolve_api_key();

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            client,
            config,
            max_retries: 3,
        })
    }

    /// Set maximum retry attempts (default: 3)
    pub fn with_max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }

    #[instrument(skip(self, messages), fields(model = %self.config.model, provider = ?self.config.provider))]
    pub async fn chat(&self, messages: Vec<ChatMessage>) -> Result<String> {
        debug!(message_count = messages.len(), "Sending chat request");
        
        let request = ChatRequest {
            model: self.config.model.clone(),
            messages,
            max_tokens: Some(self.config.max_tokens),
            temperature: Some(self.config.temperature),
        };

        let response = self.send_request_with_retry(&request).await?;

        if let Some(ref usage) = response.usage {
            debug!(
                prompt_tokens = usage.prompt_tokens,
                completion_tokens = usage.completion_tokens,
                total_tokens = usage.total_tokens,
                "API response received"
            );
        }

        response
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .context("No response content from API")
    }

    #[instrument(skip(self, request), fields(attempt_limit = self.max_retries))]
    async fn send_request_with_retry(&self, request: &ChatRequest) -> Result<ChatResponse> {
        let mut attempt = 0u32;
        let max_retries = self.max_retries;

        loop {
            attempt += 1;
            match self.send_request(request).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    let err_str = e.to_string();
                    if is_retryable_error(&err_str) && attempt <= max_retries {
                        let delay = exponential_backoff_delay(attempt);
                        warn!(
                            attempt,
                            max_retries,
                            error = %err_str,
                            delay_ms = delay.as_millis() as u64,
                            "API request failed, retrying"
                        );
                        tokio::time::sleep(delay).await;
                    } else {
                        return Err(e);
                    }
                }
            }
        }
    }

    async fn send_request(&self, request: &ChatRequest) -> Result<ChatResponse> {
        let url = format!("{}/chat/completions", self.config.get_api_base());

        let mut req = self.client.post(&url).json(request);

        // Add authentication header based on provider
        if let Some(ref api_key) = self.config.api_key {
            match self.config.provider {
                ApiProvider::Anthropic => {
                    req = req.header("x-api-key", api_key);
                    req = req.header("anthropic-version", "2023-06-01");
                }
                _ => {
                    req = req.header("Authorization", format!("Bearer {}", api_key));
                }
            }
        }

        let response = req.send().await.context("Failed to send request to API")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            let user_message = format_api_error(status.as_u16(), &error_text, &self.config);
            bail!("{}", user_message);
        }

        response
            .json::<ChatResponse>()
            .await
            .context("Failed to parse API response")
    }

    pub async fn test_connection(&self) -> Result<()> {
        let messages = vec![ChatMessage {
            role: "user".to_string(),
            content: "Say 'ok' if you can hear me.".to_string(),
        }];

        self.chat(messages).await?;
        Ok(())
    }
}

/// Check if an error is retryable (rate limit, server error, timeout)
fn is_retryable_error(err: &str) -> bool {
    let retryable_patterns = [
        "status 429",   // Rate limit
        "status 500",   // Internal server error
        "status 502",   // Bad gateway
        "status 503",   // Service unavailable
        "status 504",   // Gateway timeout
        "timeout",      // Request timeout
        "connection",   // Connection errors
        "ETIMEDOUT",    // Network timeout
        "ECONNRESET",   // Connection reset
        "ECONNREFUSED", // Connection refused
        "temporarily",  // Temporary errors
        "overloaded",   // Server overloaded
    ];

    let err_lower = err.to_lowercase();
    retryable_patterns
        .iter()
        .any(|p| err_lower.contains(&p.to_lowercase()))
}

/// Calculate exponential backoff delay: 500ms, 1s, 2s, 4s, ... capped at 30s
fn exponential_backoff_delay(attempt: u32) -> Duration {
    let base_ms = 500u64;
    let delay_ms = base_ms * 2u64.pow(attempt.saturating_sub(1));
    Duration::from_millis(delay_ms.min(30_000))
}

/// Format API errors with actionable suggestions
fn format_api_error(status: u16, error_text: &str, config: &ApiConfig) -> String {
    let provider = format!("{:?}", config.provider);
    
    match status {
        400 => {
            let mut msg = format!("Bad Request (400): {}", error_text);
            if error_text.contains("model") || error_text.contains("does not exist") {
                msg.push_str(&format!(
                    "\n\n💡 Suggestion: The model '{}' may not exist or is unavailable.\n   Try: kyogoku config set api.model <valid-model-name>",
                    config.model
                ));
            }
            if error_text.contains("max_tokens") || error_text.contains("context_length") {
                msg.push_str("\n\n💡 Suggestion: Reduce max_tokens or use a model with larger context window.");
            }
            msg
        }
        401 => {
            format!(
                "Authentication Failed (401): Invalid or missing API key\n\n\
                 💡 Suggestions:\n   \
                 1. Check your {} API key is correct\n   \
                 2. If using ENV_VAR, ensure the environment variable is set:\n      \
                    export {}_API_KEY=\"your-key\"\n   \
                 3. Verify key at your provider's dashboard",
                provider,
                provider.to_uppercase().replace(" ", "_")
            )
        }
        403 => {
            format!(
                "Access Forbidden (403): Your API key doesn't have permission for this action\n\n\
                 💡 Suggestions:\n   \
                 1. Check if the model '{}' requires special access\n   \
                 2. Verify your account has the necessary permissions\n   \
                 3. Some models require a paid subscription",
                config.model
            )
        }
        404 => {
            format!(
                "Not Found (404): The API endpoint or model doesn't exist\n\n\
                 💡 Suggestions:\n   \
                 1. Check if model '{}' exists for {}\n   \
                 2. Verify the API base URL is correct: {}\n   \
                 3. The model may have been deprecated or renamed",
                config.model, provider, config.get_api_base()
            )
        }
        429 => {
            format!(
                "Rate Limited (429): Too many requests\n\n\
                 💡 Suggestions:\n   \
                 1. Wait a few minutes before retrying\n   \
                 2. Reduce batch_size in config (current: lower values = fewer parallel requests)\n   \
                 3. Check your {} usage quotas and billing\n   \
                 4. Consider using a different API key or upgrading your plan",
                provider
            )
        }
        500 => {
            format!(
                "Server Error (500): {} API is experiencing issues\n\n\
                 💡 Suggestions:\n   \
                 1. Wait and retry (automatic retry is enabled)\n   \
                 2. Check {} status page for outages\n   \
                 3. Try a different model if available",
                provider, provider
            )
        }
        502 | 503 | 504 => {
            format!(
                "Service Unavailable ({status}): {} API is temporarily overloaded\n\n\
                 💡 Suggestions:\n   \
                 1. Automatic retry will happen shortly\n   \
                 2. If persistent, try again in a few minutes\n   \
                 3. Check {} status page for incidents",
                provider, provider
            )
        }
        _ => {
            let mut msg = format!("API Error ({status}): {error_text}");
            if error_text.contains("quota") || error_text.contains("limit") {
                msg.push_str(&format!(
                    "\n\n💡 This may be a usage limit issue. Check your {} account."
                , provider));
            }
            if error_text.contains("content") && error_text.contains("policy") {
                msg.push_str("\n\n💡 Content may have been flagged by safety filters. Review the source text.");
            }
            msg
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ApiConfig;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn test_chat_request_serialization() {
        let request = ChatRequest {
            model: "gpt-4o".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
            max_tokens: Some(100),
            temperature: Some(0.3),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("gpt-4o"));
        assert!(json.contains("Hello"));
    }

    #[tokio::test]
    async fn test_chat_success_with_mock_server() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .and(header("authorization", "Bearer test-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "chatcmpl-test",
                "choices": [{
                    "message": {"role": "assistant", "content": "你好，世界"},
                    "finish_reason": "stop"
                }],
                "usage": {"prompt_tokens": 10, "completion_tokens": 6, "total_tokens": 16}
            })))
            .expect(1)
            .mount(&server)
            .await;

        let config = ApiConfig {
            api_base: Some(format!("{}/v1", server.uri())),
            api_key: Some("test-key".to_string()),
            model: "mock-model".to_string(),
            ..ApiConfig::default()
        };

        let client = ApiClient::new(config).unwrap();
        let response = client
            .chat(vec![ChatMessage {
                role: "user".to_string(),
                content: "hello".to_string(),
            }])
            .await
            .unwrap();

        assert_eq!(response, "你好，世界");
    }

    #[tokio::test]
    async fn test_chat_api_error_surface() {
        let server = MockServer::start().await;
        // Use status 401 (Unauthorized) which is not retryable
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(401).set_body_string("unauthorized"))
            .expect(1)
            .mount(&server)
            .await;

        let config = ApiConfig {
            api_base: Some(format!("{}/v1", server.uri())),
            api_key: Some("test-key".to_string()),
            model: "mock-model".to_string(),
            ..ApiConfig::default()
        };
        let client = ApiClient::new(config).unwrap().with_max_retries(0);

        let err = client
            .chat(vec![ChatMessage {
                role: "user".to_string(),
                content: "hello".to_string(),
            }])
            .await
            .unwrap_err();

        let msg = err.to_string();
        // Error message now includes actionable suggestions
        assert!(msg.contains("Authentication Failed") || msg.contains("401"));
    }

    #[tokio::test]
    async fn test_chat_missing_content_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "chatcmpl-test",
                "choices": [{
                    "message": {"role": "assistant", "content": null},
                    "finish_reason": "stop"
                }],
                "usage": {"prompt_tokens": 2, "completion_tokens": 0, "total_tokens": 2}
            })))
            .expect(1)
            .mount(&server)
            .await;

        let config = ApiConfig {
            api_base: Some(format!("{}/v1", server.uri())),
            api_key: Some("test-key".to_string()),
            model: "mock-model".to_string(),
            ..ApiConfig::default()
        };
        let client = ApiClient::new(config).unwrap();

        let err = client
            .chat(vec![ChatMessage {
                role: "user".to_string(),
                content: "hello".to_string(),
            }])
            .await
            .unwrap_err();
        assert!(err.to_string().contains("No response content from API"));
    }

    #[tokio::test]
    async fn test_retry_on_server_error() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicU32, Ordering};

        let server = MockServer::start().await;
        let call_count = Arc::new(AtomicU32::new(0));
        let call_count_clone = call_count.clone();

        // Fail first 2 attempts, succeed on 3rd
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(move |_: &wiremock::Request| {
                let count = call_count_clone.fetch_add(1, Ordering::SeqCst);
                if count < 2 {
                    ResponseTemplate::new(503).set_body_string("service unavailable")
                } else {
                    ResponseTemplate::new(200).set_body_json(serde_json::json!({
                        "id": "chatcmpl-test",
                        "choices": [{
                            "message": {"role": "assistant", "content": "success"},
                            "finish_reason": "stop"
                        }],
                        "usage": {"prompt_tokens": 2, "completion_tokens": 1, "total_tokens": 3}
                    }))
                }
            })
            .mount(&server)
            .await;

        let config = ApiConfig {
            api_base: Some(format!("{}/v1", server.uri())),
            api_key: Some("test-key".to_string()),
            model: "mock-model".to_string(),
            ..ApiConfig::default()
        };
        let client = ApiClient::new(config).unwrap().with_max_retries(3);

        let response = client
            .chat(vec![ChatMessage {
                role: "user".to_string(),
                content: "hello".to_string(),
            }])
            .await
            .unwrap();

        assert_eq!(response, "success");
        assert_eq!(call_count.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn test_is_retryable_error() {
        assert!(is_retryable_error("status 429: rate limit"));
        assert!(is_retryable_error("API request failed with status 503"));
        assert!(is_retryable_error("connection timeout"));
        assert!(!is_retryable_error("status 401: unauthorized"));
        assert!(!is_retryable_error("invalid JSON"));
    }

    #[test]
    fn test_exponential_backoff_delay() {
        assert_eq!(exponential_backoff_delay(1), Duration::from_millis(500));
        assert_eq!(exponential_backoff_delay(2), Duration::from_millis(1000));
        assert_eq!(exponential_backoff_delay(3), Duration::from_millis(2000));
        assert_eq!(exponential_backoff_delay(10), Duration::from_secs(30)); // Capped at 30s
    }
}
