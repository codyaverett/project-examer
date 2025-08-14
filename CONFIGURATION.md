# Project Examer Configuration Guide

## Configuration File Locations

Project Examer looks for configuration files in this priority order:

1. **Custom path** (specified with `--config` flag)
2. **User's home directory**: `~/.project-examer.toml`
3. **Built-in defaults** (if no config file found)

## Quick Setup

### 1. Generate Configuration File
```bash
# Create config at default location (~/.project-examer.toml)
project-examer config

# Or specify custom location
project-examer config --output /path/to/my-config.toml
```

### 2. Set API Key
Choose one method:

**Method A: Environment Variable (Recommended)**
```bash
export OPENAI_API_KEY="your-api-key-here"
export ANTHROPIC_API_KEY="your-claude-key-here"
```

**Method B: Configuration File**
Edit `~/.project-examer.toml`:
```toml
[llm]
api_key = "your-api-key-here"
```

### 3. Test Configuration
```bash
cargo run --example config_example
```

## Complete Configuration Reference

```toml
# Project Examer Configuration File
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
```

## LLM Provider Setup

### OpenAI (GPT Models)
1. Get API key from https://platform.openai.com/
2. Set environment variable: `export OPENAI_API_KEY="sk-..."`
3. Configure model (gpt-4, gpt-3.5-turbo, etc.)

### Anthropic (Claude Models)
1. Get API key from https://console.anthropic.com/
2. Set environment variable: `export ANTHROPIC_API_KEY="sk-..."`
3. Configure model (claude-3-sonnet, claude-3-haiku, etc.)

### Ollama (Local Models)
1. Install Ollama: https://ollama.ai/
2. Pull a model: `ollama pull codellama`
3. Configure in config file:
```toml
[llm]
provider = "Ollama"
base_url = "http://localhost:11434"
model = "codellama"
```

## Usage Examples

### Global Analysis (After `cargo install`)
```bash
# First time setup
project-examer config
vim ~/.project-examer.toml  # Set your preferences

# Analyze any project
cd /path/to/any/project
project-examer analyze
```

### Project-Specific Configuration
```bash
cd my-project
project-examer config --output .project-examer.toml
# Edit .project-examer.toml for project-specific settings
project-examer analyze --config .project-examer.toml
```

### CI/CD Integration
```bash
# In CI environment
export OPENAI_API_KEY="${{ secrets.OPENAI_API_KEY }}"
project-examer analyze --skip-llm  # For fast local analysis
# Or use LLM analysis for comprehensive reports
project-examer analyze --output ./ci-analysis-reports/
```

## Environment Variables

| Variable | Description | Provider |
|----------|-------------|----------|
| `OPENAI_API_KEY` | OpenAI API key | OpenAI |
| `ANTHROPIC_API_KEY` | Anthropic API key | Anthropic |
| `HOME` | User home directory (for config location) | System |
| `USERPROFILE` | Windows user profile (config location) | Windows |

## File Structure After Installation

```
~/.project-examer.toml      # Global configuration
/usr/local/bin/project-examer  # Binary (via cargo install)

# Per-project (optional)
project-root/
├── .project-examer.toml    # Project-specific config
└── analysis-output/        # Generated reports
    ├── analysis_report.html
    ├── analysis_report.json
    └── analysis_summary.md
```

## Troubleshooting

### Config File Not Found
```
ℹ️  No config file found at ~/.project-examer.toml, using defaults
💡 Run 'project-examer config' to create a default configuration file
```
**Solution**: Run `project-examer config` to create the configuration file.

### API Key Issues
```
Error: OpenAI API key not provided
```
**Solutions**:
1. Set environment variable: `export OPENAI_API_KEY="your-key"`
2. Add to config file: `api_key = "your-key"` in `[llm]` section
3. Use local Ollama instead: Set `provider = "Ollama"` in config

### Permission Errors
```
Error: Permission denied (os error 13)
```
**Solution**: Check file permissions on config file and ensure home directory is writable.

## Security Notes

- Never commit API keys to version control
- Use environment variables in CI/CD pipelines  
- The config file is stored in your home directory with normal file permissions
- Consider using different API keys for different projects/environments