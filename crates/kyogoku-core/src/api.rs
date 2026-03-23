use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

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
}

impl ApiClient {
    pub fn new(config: ApiConfig) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self { client, config })
    }

    pub async fn chat(&self, messages: Vec<ChatMessage>) -> Result<String> {
        let request = ChatRequest {
            model: self.config.model.clone(),
            messages,
            max_tokens: Some(self.config.max_tokens),
            temperature: Some(self.config.temperature),
        };

        let response = self.send_request(&request).await?;

        response
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .context("No response content from API")
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
            bail!("API request failed with status {}: {}", status, error_text);
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
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(500).set_body_string("internal error"))
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

        let msg = err.to_string();
        assert!(msg.contains("API request failed with status"));
        assert!(msg.contains("internal error"));
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
}
