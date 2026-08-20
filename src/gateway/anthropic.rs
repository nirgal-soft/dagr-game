use anyhow::{Context, Result};
use async_trait::async_trait;
use dagr_lib::agency::{
    AnthropicTransportConfig, ModelDecision, ModelOptions, ModelProfile, ModelProvider,
    ModelRequest, ModelResponse, ProviderKind, ToolCall,
};
use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue};
use serde::Deserialize;
use serde_json::{Value, json};

use super::{request_prompt, system_prompt};

pub struct AnthropicProvider {
    client: reqwest::Client,
    transport: AnthropicTransportConfig,
}

impl AnthropicProvider {
    pub fn new(transport: AnthropicTransportConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            transport,
        }
    }

    fn headers(&self) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();
        let mut api_key = HeaderValue::from_str(&self.transport.api_key)?;
        api_key.set_sensitive(true);
        headers.insert("x-api-key", api_key);
        headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        Ok(headers)
    }
}

#[async_trait]
impl ModelProvider for AnthropicProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Anthropic
    }

    async fn decide(
        &self,
        profile: &ModelProfile,
        request: &ModelRequest,
    ) -> Result<ModelResponse, Box<dyn std::error::Error + Send + Sync>> {
        let body = request_body(profile, request)?;
        let response = self
            .client
            .post(&self.transport.endpoint)
            .timeout(self.transport.timeout)
            .headers(self.headers()?)
            .json(&body)
            .send()
            .await
            .context("Anthropic model request failed")?;
        let status = response.status();
        let body = response.text().await?;
        if !status.is_success() {
            return Err(anyhow::anyhow!(
                "Anthropic model request failed with status {status}: {body}"
            )
            .into());
        }
        Ok(response_from_body(&body)?)
    }
}

fn request_body(profile: &ModelProfile, request: &ModelRequest) -> Result<Value> {
    let ModelOptions::Anthropic {
        thinking_budget_tokens,
    } = profile.options
    else {
        anyhow::bail!("Anthropic provider received incompatible model options")
    };
    let tools = request
        .tools
        .iter()
        .map(|tool| {
            json!({
              "name": tool.name,
              "description": tool.description,
              "input_schema": tool.input_schema,
            })
        })
        .collect::<Vec<_>>();
    let mut body = json!({
      "model": profile.model,
      "max_tokens": profile.max_output_tokens,
      "system": system_prompt(),
      "messages": [{
        "role": "user",
        "content": request_prompt(request)?,
      }],
    });
    if !tools.is_empty() {
        body["tools"] = Value::Array(tools);
        body["tool_choice"] = json!({"type": "auto"});
    }
    if let Some(budget_tokens) = thinking_budget_tokens {
        body["thinking"] = json!({
          "type": "enabled",
          "budget_tokens": budget_tokens,
        });
    }
    Ok(body)
}

fn response_from_body(body: &str) -> Result<ModelResponse> {
    let response: AnthropicResponse =
        serde_json::from_str(body).context("failed to parse Anthropic model response")?;
    let mut calls = Vec::new();
    let mut text = Vec::new();
    for block in response.content {
        match block {
            AnthropicContent::ToolUse { id, name, input } => {
                calls.push(ToolCall { id, name, input })
            }
            AnthropicContent::Text { text: value } if !value.trim().is_empty() => text.push(value),
            AnthropicContent::Text { .. }
            | AnthropicContent::Thinking {}
            | AnthropicContent::RedactedThinking {} => {}
        }
    }
    Ok(ModelResponse {
        decision: if calls.is_empty() {
            ModelDecision::Stop
        } else {
            ModelDecision::CallTools(calls)
        },
        visible_text: (!text.is_empty()).then(|| text.join("\n\n")),
    })
}

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContent>,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum AnthropicContent {
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    Text {
        text: String,
    },
    Thinking {},
    RedactedThinking {},
}

#[cfg(test)]
mod tests {
    use super::*;
    use dagr_lib::agency::ActorScope;

    fn request() -> ModelRequest {
        ModelRequest {
            actor: ActorScope::Gm,
            actor_context: json!({"scene":"tower"}),
            stimulus: json!({"event":"tremor"}),
            tools: vec![],
            prior_outcomes: vec![],
            remaining_tool_calls: 3,
        }
    }

    #[test]
    fn request_uses_profile_and_omits_universal_temperature() {
        let body = request_body(
            &ModelProfile {
                model: "claude-test".to_string(),
                max_output_tokens: 2_048,
                options: ModelOptions::Anthropic {
                    thinking_budget_tokens: Some(1_024),
                },
            },
            &request(),
        )
        .unwrap();
        assert_eq!(body["model"], "claude-test");
        assert_eq!(body["max_tokens"], 2_048);
        assert_eq!(body["thinking"]["budget_tokens"], 1_024);
        assert!(body.get("temperature").is_none());
        assert!(body.get("tools").is_none());
        assert!(body.get("tool_choice").is_none());
    }

    #[test]
    fn maps_visible_text_and_native_tool_calls() {
        let response = response_from_body(
            r#"{
        "content": [
          {"type":"text", "text":"The tower trembles."},
          {"type":"tool_use", "id":"call-1", "name":"front_advance", "input":{"danger_id":7}}
        ]
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
        assert_eq!(calls[0].name, "front_advance");
    }

    #[test]
    fn ignores_thinking_blocks_while_mapping_visible_output() {
        let response = response_from_body(
            r#"{
        "content": [
          {"type":"thinking", "thinking":"We need a tool.", "signature":"signed"},
          {"type":"redacted_thinking", "data":"encrypted"},
          {"type":"text", "text":"The tower trembles."},
          {"type":"tool_use", "id":"call-1", "name":"front_advance", "input":{}}
        ]
      }"#,
        )
        .unwrap();
        assert_eq!(
            response.visible_text.as_deref(),
            Some("The tower trembles.")
        );
        assert!(matches!(response.decision, ModelDecision::CallTools(_)));
    }

    #[test]
    fn text_only_response_stops() {
        let response =
            response_from_body(r#"{"content":[{"type":"text","text":"Nothing changes."}]}"#)
                .unwrap();
        assert_eq!(response.decision, ModelDecision::Stop);
        assert_eq!(response.visible_text.as_deref(), Some("Nothing changes."));
    }
}
