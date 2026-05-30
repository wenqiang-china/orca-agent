use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Top-level Orca configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrcaConfig {
    /// Active provider name
    pub provider: String,
    /// Model ID to use
    pub model: String,
    /// Provider configurations
    pub providers: std::collections::HashMap<String, crate::provider_config::ProviderConfig>,
    /// Sandbox settings
    pub sandbox: SandboxConfig,
    /// Budget limits
    pub budget: BudgetConfig,
    /// Context management settings
    pub context: ContextConfig,
    /// Log level
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Enable OS-level sandboxing
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Command execution timeout in seconds
    #[serde(default = "default_exec_timeout")]
    pub exec_timeout_secs: u64,
    /// Maximum exec timeout in seconds (hard limit)
    #[serde(default = "default_max_exec_timeout")]
    pub max_exec_timeout_secs: u64,
    /// Network policy: "denied", "restricted", "full"
    #[serde(default = "default_network_policy")]
    pub network_policy: String,
    /// Read-only paths (in addition to default system paths)
    #[serde(default)]
    pub read_only_paths: Vec<PathBuf>,
    /// Writable paths (project root is always writable)
    #[serde(default)]
    pub writable_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetConfig {
    /// Maximum USD to spend per session
    #[serde(default = "default_max_budget")]
    pub max_session_budget_usd: f64,
    /// Maximum iterations (model calls) per session
    #[serde(default = "default_max_iterations")]
    pub max_iterations: u32,
    /// Rate limit: max requests per minute
    #[serde(default = "default_rate_limit")]
    pub rate_limit_per_minute: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextConfig {
    /// Maximum context window tokens
    #[serde(default = "default_max_context_tokens")]
    pub max_context_tokens: usize,
    /// Threshold (0.0-1.0) at which to start compressing
    #[serde(default = "default_compress_threshold")]
    pub compress_threshold: f64,
    /// Number of checkpoints to keep
    #[serde(default = "default_max_checkpoints")]
    pub max_checkpoints: usize,
    /// Whether to use Flash model for summarization
    #[serde(default = "default_true")]
    pub use_flash_summary: bool,
}

fn default_log_level() -> String { "info".to_string() }
fn default_true() -> bool { true }
fn default_exec_timeout() -> u64 { 120 }
fn default_max_exec_timeout() -> u64 { 600 }
fn default_network_policy() -> String { "denied".to_string() }
fn default_max_budget() -> f64 { 10.0 }
fn default_max_iterations() -> u32 { 200 }
fn default_rate_limit() -> u32 { 60 }
fn default_max_context_tokens() -> usize { 128_000 }
fn default_compress_threshold() -> f64 { 0.75 }
fn default_max_checkpoints() -> usize { 10 }

impl Default for OrcaConfig {
    fn default() -> Self {
        Self {
            provider: "deepseek".to_string(),
            model: "deepseek-chat".to_string(),
            providers: std::collections::HashMap::new(),
            sandbox: SandboxConfig {
                enabled: true,
                exec_timeout_secs: 120,
                max_exec_timeout_secs: 600,
                network_policy: "denied".to_string(),
                read_only_paths: Vec::new(),
                writable_paths: Vec::new(),
            },
            budget: BudgetConfig {
                max_session_budget_usd: 10.0,
                max_iterations: 200,
                rate_limit_per_minute: 60,
            },
            context: ContextConfig {
                max_context_tokens: 128_000,
                compress_threshold: 0.75,
                max_checkpoints: 10,
                use_flash_summary: true,
            },
            log_level: "info".to_string(),
        }
    }
}

impl OrcaConfig {
    /// Load config from file, falling back to defaults
    pub fn load() -> Result<Self, anyhow::Error> {
        let config_path = Self::config_path()?;
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let config: Self = toml::from_str(&content)?;
            Ok(config)
        } else {
            Ok(Self::default())
        }
    }

    /// Save config to file
    pub fn save(&self) -> Result<(), anyhow::Error> {
        let config_path = Self::config_path()?;
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&config_path, content)?;
        Ok(())
    }

    /// Get the default config file path
    pub fn config_path() -> Result<PathBuf, anyhow::Error> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("cannot determine config directory"))?;
        Ok(config_dir.join("orca").join("config.toml"))
    }

    /// Get the data directory for checkpoints, events, etc.
    pub fn data_dir() -> Result<PathBuf, anyhow::Error> {
        let data_dir = dirs::data_dir()
            .ok_or_else(|| anyhow::anyhow!("cannot determine data directory"))?;
        Ok(data_dir.join("orca"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = OrcaConfig::default();
        assert_eq!(config.provider, "deepseek");
        assert_eq!(config.model, "deepseek-chat");
        assert!(config.sandbox.enabled);
        assert_eq!(config.budget.max_iterations, 200);
    }

    #[test]
    fn test_serialize_roundtrip() {
        let config = OrcaConfig::default();
        let toml_str = toml::to_string(&config).unwrap();
        let parsed: OrcaConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.provider, config.provider);
        assert_eq!(parsed.model, config.model);
    }
}