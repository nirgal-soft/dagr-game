use std::sync::Arc;

use anyhow::{Context, Result};
use dagr_lib::agency::{
    AgentRuntime, ModelProvider, ModelRequest, ModelRuntimeConfig, ProviderTransportConfig,
    StructuredOutputProvider, StructuredOutputRuntime,
};

mod anthropic;
mod ollama;

use anthropic::AnthropicProvider;
use ollama::OllamaProvider;

pub struct ModelRuntimes {
    pub agent: Arc<AgentRuntime>,
    pub structured: Arc<StructuredOutputRuntime>,
}

impl ModelRuntimes {
    pub fn from_env() -> Result<Self> {
        Self::from_config(ModelRuntimeConfig::from_env()?)
    }

    fn from_config(config: ModelRuntimeConfig) -> Result<Self> {
        let agent_provider: Arc<dyn ModelProvider> = match &config.agent.transport {
            ProviderTransportConfig::Anthropic(transport) => {
                Arc::new(AnthropicProvider::new(transport.clone()))
            }
            ProviderTransportConfig::Ollama(transport) => Arc::new(
                OllamaProvider::new(transport.clone()).context("invalid agent Ollama transport")?,
            ),
        };
        let structured_provider: Arc<dyn StructuredOutputProvider> = Arc::new(
            OllamaProvider::new(config.structured.transport.clone())
                .context("invalid structured-output Ollama transport")?,
        );
        Ok(Self {
            agent: Arc::new(
                AgentRuntime::new(config.agent, agent_provider)
                    .context("invalid agent runtime configuration")?,
            ),
            structured: Arc::new(
                StructuredOutputRuntime::new(config.structured, structured_provider)
                    .context("invalid structured-output runtime configuration")?,
            ),
        })
    }
}

pub(super) fn request_prompt(request: &ModelRequest) -> Result<String> {
    Ok(format!(
        "ACTOR SCOPE:\n{}\n\nSTIMULUS:\n{}\n\nCANONICAL ACTOR CONTEXT:\n{}\n\nPRIOR TOOL OUTCOMES:\n{}\n\nREMAINING TOOL CALLS: {}",
        serde_json::to_string(&request.actor)?,
        serde_json::to_string_pretty(&request.stimulus)?,
        serde_json::to_string_pretty(&request.actor_context)?,
        serde_json::to_string_pretty(&request.prior_outcomes)?,
        request.remaining_tool_calls,
    ))
}

pub(super) fn system_prompt() -> &'static str {
    "You animate the scoped GM, NPC, or Faction actor in the DAGR engine. Stay \
   faithful to the supplied actor scope and canonical context. Canonical facts are \
   only those present in context or returned by tools. Choose tools to act; do not \
   claim that an attempted action succeeded. Prose is non-authoritative but may be \
   shown to the user; use it to explain choices and narrate committed tool outcomes. \
   Stop when no further tool use is warranted."
}

#[cfg(test)]
mod tests {
    use super::*;
    use dagr_lib::agency::{AgentRole, ProviderKind};

    #[test]
    fn client_factory_constructs_default_local_runtimes_without_a_network_probe() {
        let config = ModelRuntimeConfig::from_vars(Vec::new()).unwrap();
        let runtimes = ModelRuntimes::from_config(config).unwrap();

        let agent = runtimes.agent.diagnostics(&AgentRole::Gm);
        assert_eq!(agent.provider, ProviderKind::Ollama);
        assert_eq!(agent.model, "qwen3:4b");

        let structured = runtimes.structured.diagnostics();
        assert_eq!(structured.provider, ProviderKind::Ollama);
        assert_eq!(structured.model, "qwen3:4b");
    }

    #[test]
    fn client_factory_constructs_documented_anthropic_runtime() {
        let config = ModelRuntimeConfig::from_vars([
            ("DAGR_AGENT_PROVIDER".to_string(), "anthropic".to_string()),
            ("ANTHROPIC_API_KEY".to_string(), "test-key".to_string()),
            (
                "DAGR_AGENT_PRIMARY_MODEL".to_string(),
                "claude-haiku-4-5-20251001".to_string(),
            ),
            (
                "DAGR_AGENT_ECONOMY_MODEL".to_string(),
                "claude-haiku-4-5-20251001".to_string(),
            ),
        ])
        .unwrap();
        let runtimes = ModelRuntimes::from_config(config).unwrap();

        let agent = runtimes.agent.diagnostics(&AgentRole::Gm);
        assert_eq!(agent.provider, ProviderKind::Anthropic);
        assert_eq!(agent.model, "claude-haiku-4-5-20251001");
    }
}
