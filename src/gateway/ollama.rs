use anyhow::{Context, Result};
use async_trait::async_trait;
use dagr_lib::agency::{
    ModelDecision, ModelOptions, ModelProfile, ModelProvider, ModelRequest, ModelResponse,
    OllamaTransportConfig, ProviderKind, StructuredOutputConfig, StructuredOutputProvider,
    StructuredOutputRequest, ToolCall,
};
use serde::Deserialize;
use serde_json::{Value, json};

use super::{request_prompt, system_prompt};

pub struct OllamaProvider {
    client: reqwest::Client,
    endpoint: reqwest::Url,
    timeout: std::time::Duration,
}

impl OllamaProvider {
    pub fn new(transport: OllamaTransportConfig) -> Result<Self> {
        let endpoint = reqwest::Url::parse(&format!(
            "{}/api/chat",
            transport.base_url.trim_end_matches('/')
        ))
        .context("invalid Ollama chat endpoint")?;
        Ok(Self {
            client: reqwest::Client::new(),
            endpoint,
            timeout: transport.timeout,
        })
    }

    async fn post(&self, body: &Value) -> Result<String> {
        let response = self
            .client
            .post(self.endpoint.clone())
            .timeout(self.timeout)
            .json(body)
            .send()
            .await
            .context("Ollama model request failed")?;
        let status = response.status();
        let body = response.text().await?;
        if !status.is_success() {
            anyhow::bail!("Ollama model request failed with status {status}: {body}");
        }
        Ok(body)
    }
}

#[async_trait]
impl ModelProvider for OllamaProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Ollama
    }

    async fn decide(
        &self,
        profile: &ModelProfile,
        request: &ModelRequest,
    ) -> Result<ModelResponse, Box<dyn std::error::Error + Send + Sync>> {
        let body = agent_request_body(profile, request)?;
        Ok(agent_response_from_body(&self.post(&body).await?)?)
    }
}

#[async_trait]
impl StructuredOutputProvider for OllamaProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Ollama
    }

    async fn generate(
        &self,
        config: &StructuredOutputConfig,
        request: &StructuredOutputRequest,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let body = structured_request_body(config, request);
        let response = self.post(&body).await?;
        Ok(structured_response_from_body(&response)?)
    }
}

fn agent_request_body(profile: &ModelProfile, request: &ModelRequest) -> Result<Value> {
    let ModelOptions::Ollama { think } = profile.options else {
        anyhow::bail!("Ollama provider received incompatible model options")
    };
    let tools = request
        .tools
        .iter()
        .map(|tool| {
            json!({
              "type": "function",
              "function": {
                "name": tool.name,
                "description": tool.description,
                "parameters": tool.input_schema,
              }
            })
        })
        .collect::<Vec<_>>();
    let mut body = json!({
      "model": profile.model,
      "stream": false,
      "think": think,
      "messages": [
        {"role": "system", "content": system_prompt()},
        {"role": "user", "content": request_prompt(request)?},
      ],
      "options": {"num_predict": profile.max_output_tokens},
    });
    if !tools.is_empty() {
        body["tools"] = Value::Array(tools);
    }
    Ok(body)
}

fn structured_request_body(
    config: &StructuredOutputConfig,
    request: &StructuredOutputRequest,
) -> Value {
    json!({
      "model": config.model,
      "stream": false,
      "think": false,
      "format": request.schema,
      "messages": [
        {"role": "system", "content": request.system},
        {"role": "user", "content": request.prompt},
      ],
      "options": {"num_ctx": config.context_tokens},
    })
}

fn agent_response_from_body(body: &str) -> Result<ModelResponse> {
    let response: OllamaResponse =
        serde_json::from_str(body).context("failed to parse Ollama model response")?;
    let calls = response
        .message
        .tool_calls
        .into_iter()
        .map(|call| ToolCall {
            id: format!("ollama-{:032x}", rand::random::<u128>()),
            name: call.function.name,
            input: call.function.arguments,
        })
        .collect::<Vec<_>>();
    let visible_text =
        (!response.message.content.trim().is_empty()).then_some(response.message.content);
    Ok(ModelResponse {
        decision: if calls.is_empty() {
            ModelDecision::Stop
        } else {
            ModelDecision::CallTools(calls)
        },
        visible_text,
    })
}

fn structured_response_from_body(body: &str) -> Result<Value> {
    let response: OllamaResponse =
        serde_json::from_str(body).context("failed to parse Ollama structured-output response")?;
    serde_json::from_str(&response.message.content)
        .context("Ollama response content did not match the requested JSON schema")
}

#[derive(Deserialize)]
struct OllamaResponse {
    message: OllamaMessage,
}

#[derive(Deserialize)]
struct OllamaMessage {
    #[serde(default)]
    content: String,
    #[serde(default)]
    tool_calls: Vec<OllamaToolCall>,
}

#[derive(Deserialize)]
struct OllamaToolCall {
    function: OllamaFunctionCall,
}

#[derive(Deserialize)]
struct OllamaFunctionCall {
    name: String,
    arguments: Value,
}

#[cfg(test)]
mod tests {
    use super::*;
    use dagr_lib::agency::{ActorScope, ToolView};

    fn request() -> ModelRequest {
        ModelRequest {
            actor: ActorScope::Gm,
            actor_context: json!({"scene":"tower"}),
            stimulus: json!({"event":"tremor"}),
            tools: vec![ToolView {
                name: "front_advance".to_string(),
                description: "Advance a danger.".to_string(),
                input_schema: json!({"type":"object"}),
            }],
            prior_outcomes: vec![],
            remaining_tool_calls: 3,
        }
    }

    #[test]
    fn agent_request_uses_profile_and_native_tool_calling() {
        let profile = ModelProfile {
            model: "qwen-test".to_string(),
            max_output_tokens: 2_048,
            options: ModelOptions::Ollama { think: true },
        };
        let body = agent_request_body(&profile, &request()).unwrap();
        assert_eq!(body["model"], "qwen-test");
        assert_eq!(body["think"], true);
        assert_eq!(body["options"]["num_predict"], 2_048);
        assert_eq!(body["stream"], false);
        assert_eq!(body["tools"][0]["type"], "function");
        assert_eq!(body["tools"][0]["function"]["name"], "front_advance");

        let mut no_tools = request();
        no_tools.tools.clear();
        let body = agent_request_body(&profile, &no_tools).unwrap();
        assert!(body.get("tools").is_none());
    }

    #[test]
    fn maps_visible_text_and_native_tool_calls() {
        let response = agent_response_from_body(
            r#"{
        "message": {
          "role": "assistant",
          "content": "The tower trembles.",
          "tool_calls": [{"function":{"name":"front_advance","arguments":{"danger_id":7}}}]
        },
        "done": true,
        "done_reason": "stop"
      }"#,
        )
        .unwrap();
        assert_eq!(
            response.visible_text.as_deref(),
            Some("The tower trembles.")
        );
        let ModelDecision::CallTools(calls) = response.decision else {
            panic!("expected tool calls")
        };
        let first_id = calls[0].id.clone();
        assert!(first_id.starts_with("ollama-"));
        assert_eq!(first_id.len(), 39);
        assert_eq!(calls[0].name, "front_advance");
        let second_response = agent_response_from_body(
            r#"{"message":{"tool_calls":[{"function":{"name":"front_advance","arguments":{}}}]}}"#,
        )
        .unwrap();
        let ModelDecision::CallTools(second_calls) = second_response.decision else {
            panic!("expected tool calls")
        };
        assert_ne!(first_id, second_calls[0].id);
    }

    #[test]
    fn text_only_response_stops() {
        let response = agent_response_from_body(
            r#"{"message":{"role":"assistant","content":"Nothing changes."},"done":true}"#,
        )
        .unwrap();
        assert_eq!(response.decision, ModelDecision::Stop);
        assert_eq!(response.visible_text.as_deref(), Some("Nothing changes."));
    }

    #[test]
    fn structured_request_uses_schema_and_context_without_thinking() {
        let config = StructuredOutputConfig {
            transport: OllamaTransportConfig {
                base_url: "http://localhost:11434".to_string(),
                timeout: std::time::Duration::from_secs(60),
            },
            model: "qwen-test".to_string(),
            context_tokens: 8_192,
        };
        let body = structured_request_body(
            &config,
            &StructuredOutputRequest {
                system: "Return data.".to_string(),
                prompt: "Describe the ruin.".to_string(),
                schema: json!({"type":"object"}),
            },
        );
        assert_eq!(body["model"], "qwen-test");
        assert_eq!(body["format"]["type"], "object");
        assert_eq!(body["options"]["num_ctx"], 8_192);
        assert_eq!(body["think"], false);
    }

    #[test]
    fn structured_response_parses_json_content() {
        let value = structured_response_from_body(
            r#"{"message":{"role":"assistant","content":"{\"name\":\"Sunken Court\"}"}}"#,
        )
        .unwrap();
        assert_eq!(value, json!({"name":"Sunken Court"}));
    }
}
