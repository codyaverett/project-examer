use serde::{Deserialize, Serialize};
use std::{env, path::PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub target_directory: PathBuf,
    pub ignore_patterns: Vec<String>,
    pub file_extensions: Vec<String>,
    pub max_file_size: usize,
    pub llm: LLMConfig,
    pub analysis: AnalysisConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMConfig {
    pub provider: LLMProvider,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub model: String,
    pub max_tokens: usize,
    pub temperature: f32,
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LLMProvider {
    OpenAI,
    Ollama,
    Anthropic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisConfig {
    pub include_dependencies: bool,
    pub include_function_calls: bool,
    pub include_architecture_patterns: bool,
    pub include_security_analysis: bool,
    pub max_depth: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            target_directory: PathBuf::from("."),
            ignore_patterns: vec![
                "node_modules".to_string(),
                ".git".to_string(),
                "target".to_string(),
                "build".to_string(),
                "dist".to_string(),
                "*.log".to_string(),
                ".env".to_string(),
                ".env.*".to_string(),
                "*.min.js".to_string(),
                "*.map".to_string(),
                "test-*".to_string(),
                "test_*".to_string(),
            ],
            file_extensions: vec![
                "rs".to_string(),
                "js".to_string(),
                "ts".to_string(),
                "tsx".to_string(),
                "jsx".to_string(),
                "py".to_string(),
                "java".to_string(),
                "go".to_string(),
                "cpp".to_string(),
                "c".to_string(),
                "h".to_string(),
                "md".to_string(),
                "txt".to_string(),
                "toml".to_string(),
                "yaml".to_string(),
                "yml".to_string(),
                "json".to_string(),
                "html".to_string(),
                "css".to_string(),
            ],
            max_file_size: 1024 * 1024, // 1MB
            llm: LLMConfig {
                provider: LLMProvider::OpenAI,
                api_key: None,
                base_url: None,
                model: "gpt-4".to_string(),
                max_tokens: 4000,
                temperature: 0.1,
                timeout_seconds: 300,
            },
            analysis: AnalysisConfig {
                include_dependencies: true,
                include_function_calls: true,
                include_architecture_patterns: true,
                include_security_analysis: false,
                max_depth: 10,
            },
        }
    }
}

impl Config {
    /// Get the default config file path (~/.project-examer.toml)
    pub fn default_config_path() -> crate::Result<PathBuf> {
        let home_dir = env::var("HOME")
            .or_else(|_| env::var("USERPROFILE"))
            .map_err(|_| anyhow::anyhow!("Could not determine home directory"))?;
        Ok(PathBuf::from(home_dir).join(".project-examer.toml"))
    }

    /// Load config from file, falling back to defaults if file doesn't exist
    pub fn load() -> crate::Result<Self> {
        let config_path = Self::default_config_path()?;
        
        let mut config = if config_path.exists() {
            println!("📝 Loading configuration from: {}", config_path.display());
            Self::from_file(&config_path)?
        } else {
            println!("ℹ️  No config file found at {}, using defaults", config_path.display());
            println!("💡 Run 'project-examer config' to create a default configuration file");
            Self::default()
        };
        
        // Override API key from environment variables if not set in config
        if config.llm.api_key.is_none() {
            config.llm.api_key = match config.llm.provider {
                LLMProvider::OpenAI => env::var("OPENAI_API_KEY").ok(),
                LLMProvider::Anthropic => env::var("ANTHROPIC_API_KEY").ok(),
                LLMProvider::Ollama => None, // Ollama typically doesn't need API keys
            };
        }
        
        Ok(config)
    }

    /// Load config from a specific file path
    pub fn from_file(path: &PathBuf) -> crate::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save config to a file
    pub fn to_file(&self, path: &PathBuf) -> crate::Result<()> {
        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Save config to the default location
    pub fn save_default(&self) -> crate::Result<()> {
        let config_path = Self::default_config_path()?;
        self.to_file(&config_path)
    }

    /// Create a config file with all available options documented
    pub fn create_documented_config() -> String {
        format!(r#"# Project Examer Configuration File
# This file configures how project-examer analyzes your codebase

# Target directory to analyze (defaults to current directory)
target_directory = "."

# Patterns to ignore during file discovery
ignore_patterns = [
    "node_modules",
    ".git", 
    "target",
    "build",
    "dist",
    "*.log",
    ".env",
    ".env.*",
    "*.min.js",
    "*.map"
]

# File extensions to include in analysis
file_extensions = [
    "rs", "js", "ts", "tsx", "jsx", "py", "java", "go", 
    "cpp", "c", "h", "php", "rb", "cs", "swift", "kt",
    "scala", "clj", "hs", "ml", "elm", "ex", "erl", "dart",
    "lua", "r", "pl", "sh", "sql", "html", "css", "scss"
]

# Maximum file size to analyze (in bytes, default 1MB)
max_file_size = 1048576

[llm]
# LLM Provider: "OpenAI", "Ollama", or "Anthropic"
provider = "OpenAI"

# API key for the provider (can also be set via environment variables)
# OpenAI: OPENAI_API_KEY
# Anthropic: ANTHROPIC_API_KEY  
# api_key = "your-api-key-here"

# Base URL (mainly for Ollama local instances)
# base_url = "http://localhost:11434"

# Model to use
model = "gpt-4"

# Maximum tokens for LLM responses
max_tokens = 4000

# Temperature for LLM responses (0.0 = deterministic, 1.0 = creative)
temperature = 0.1

# Request timeout in seconds (default: 300 seconds / 5 minutes)
timeout_seconds = 300

[analysis]
# Include dependency analysis
include_dependencies = true

# Include function call analysis
include_function_calls = true

# Include architecture pattern detection
include_architecture_patterns = true

# Include security vulnerability analysis
include_security_analysis = false

# Maximum depth for dependency traversal
max_depth = 10
"#)
    }
}