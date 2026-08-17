use anyhow::{Context, Result};
use async_trait::async_trait;
use dagr_lib::agency::{ModelDecision, ModelProvider, ModelRequest, ModelResponse, ToolCall};
use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue};
use serde::Deserialize;
use serde_json::{Value, json};

pub const DEFAULT_MODEL: &str = "claude-haiku-4-5-20251001";
const DEFAULT_ENDPOINT: &str = "https://api.anthropic.com/v1/messages";

pub struct HostedGateway {
  client: reqwest::Client,
  api_key: String,
  model: String,
  endpoint: String,
}

impl HostedGateway {
  pub fn from_env() -> Result<Option<Self>> {
    let Some(api_key) = std::env::var("ANTHROPIC_API_KEY")
      .ok()
      .filter(|key| !key.trim().is_empty())
    else {
      return Ok(None);
    };
    Ok(Some(Self {
      client: reqwest::Client::new(),
      api_key,
      model: std::env::var("DAGR_LLM_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string()),
      endpoint: std::env::var("DAGR_LLM_ENDPOINT").unwrap_or_else(|_| DEFAULT_ENDPOINT.to_string()),
    }))
  }

  fn headers(&self) -> Result<HeaderMap> {
    let mut headers = HeaderMap::new();
    headers.insert("x-api-key", HeaderValue::from_str(&self.api_key)?);
    headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    Ok(headers)
  }
}

#[async_trait]
impl ModelProvider for HostedGateway {
  async fn decide(
    &self,
    request: &ModelRequest,
  ) -> Result<ModelResponse, Box<dyn std::error::Error + Send + Sync>> {
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
    let body = json!({
      "model": self.model,
      "max_tokens": 1024,
      "temperature": 0.4,
      "system": system_prompt(),
      "messages": [{
        "role": "user",
        "content": format!(
          "ACTOR SCOPE:\n{}\n\nSTIMULUS:\n{}\n\nCANONICAL ACTOR CONTEXT:\n{}\n\nPRIOR TOOL OUTCOMES:\n{}\n\nREMAINING TOOL CALLS: {}",
          serde_json::to_string(&request.actor)?,
          serde_json::to_string_pretty(&request.stimulus)?,
          serde_json::to_string_pretty(&request.actor_context)?,
          serde_json::to_string_pretty(&request.prior_outcomes)?,
          request.remaining_tool_calls,
        )
      }],
      "tools": tools,
      "tool_choice": {"type": "auto"},
    });
    let response = self
      .client
      .post(&self.endpoint)
      .timeout(std::time::Duration::from_secs(60))
      .headers(self.headers()?)
      .json(&body)
      .send()
      .await
      .context("hosted model request failed")?;
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
      return Err(
        anyhow::anyhow!("hosted model request failed with status {status}: {body}").into(),
      );
    }
    Ok(response_from_body(&body)?)
  }
}

fn response_from_body(body: &str) -> Result<ModelResponse> {
  let response: GatewayResponse =
    serde_json::from_str(body).context("failed to parse hosted model response")?;
  let mut calls = Vec::new();
  let mut text = Vec::new();
  for block in response.content {
    match block {
      GatewayContent::ToolUse { id, name, input } => calls.push(ToolCall { id, name, input }),
      GatewayContent::Text { text: value } if !value.trim().is_empty() => text.push(value),
      GatewayContent::Text { .. } => {}
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

fn system_prompt() -> &'static str {
  "You animate the scoped GM, NPC, or Faction actor in the DAGR engine. Stay \
   faithful to the supplied actor scope and canonical context. Canonical facts are \
   only those present in context or returned by tools. Choose tools to act; do not \
   claim that an attempted action succeeded. Prose is non-authoritative but may be \
   shown to the user; use it to explain choices and narrate committed tool outcomes. \
   Stop when no further tool use is warranted."
}

#[derive(Deserialize)]
struct GatewayResponse {
  content: Vec<GatewayContent>,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum GatewayContent {
  ToolUse {
    id: String,
    name: String,
    input: Value,
  },
  Text {
    text: String,
  },
}

#[cfg(test)]
mod tests {
  use super::*;

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
  fn text_only_response_stops() {
    let response =
      response_from_body(r#"{"content":[{"type":"text","text":"Nothing changes."}]}"#).unwrap();
    assert_eq!(response.decision, ModelDecision::Stop);
    assert_eq!(response.visible_text.as_deref(), Some("Nothing changes."));
  }
}
