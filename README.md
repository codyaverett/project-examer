# Project Examer 🔍

A fast, comprehensive system analysis tool for scanning directories and building intelligent relationships between files using AST parsing and LLM analysis.

## Features

- **🚀 Fast File Discovery**: Recursive directory scanning with configurable ignore patterns
- **🌳 AST Parsing**: Support for multiple languages using Tree-sitter
- **🕸️ Dependency Graph**: Build comprehensive dependency relationships
- **🤖 LLM Analysis**: AI-powered insights using OpenAI, Anthropic, or local Ollama
- **📊 Rich Reporting**: Generate HTML, JSON, and Markdown reports
- **⚡ Parallel Processing**: Efficient multi-threaded file processing
- **🔧 Configurable**: Fully customizable via TOML configuration

## Supported Languages

- TypeScript/JavaScript (with JSX/TSX)
- Python
- Rust
- Java
- Go
- C/C++
- And more...

## Installation

### From Source
```bash
git clone <repository-url>
cd project-examer
cargo install --path .
```

### Global Installation
```bash
# Install from crates.io
cargo install project-examer
```

After global installation, the tool is available as `project-examer` from anywhere.

## Quick Start

### Analyze a project
```bash
# Analyze current directory
project-examer analyze

# Analyze specific directory
project-examer analyze --path /path/to/project

# Skip LLM analysis for faster local-only results
project-examer analyze --skip-llm

# Use custom configuration
project-examer analyze --config custom-config.toml
```

### Generate configuration file
```bash
# Generate config at default location (~/.project-examer.toml)
project-examer config

# Generate config at custom location
project-examer config --output my-config.toml
```

## Configuration

Project Examer looks for configuration in the following order:
1. Custom path specified with `--config`
2. `~/.project-examer.toml` (user's home directory)
3. Built-in defaults

Generate a configuration file with all options documented:

```bash
project-examer config  # Creates ~/.project-examer.toml with full documentation
```

### Environment Variables

API keys can be provided via environment variables:
- `OPENAI_API_KEY` - for OpenAI GPT models
- `ANTHROPIC_API_KEY` - for Claude models

### Configuration File Structure

```toml
target_directory = "."
ignore_patterns = ["node_modules", ".git", "target", "build", "dist"]
file_extensions = ["rs", "js", "ts", "tsx", "jsx", "py", "java", "go"]
max_file_size = 1048576  # 1MB

[llm]
provider = "OpenAI"  # Options: "OpenAI", "Ollama", "Anthropic"
model = "gpt-4"
max_tokens = 4000
temperature = 0.1
# api_key = "your-key"  # Or use environment variables

[analysis]
include_dependencies = true
include_function_calls = true
include_architecture_patterns = true
include_security_analysis = false
max_depth = 10
```

## Output

Project Examer generates comprehensive analysis reports:

### 📄 Analysis Report (HTML/JSON/Markdown)
- Executive summary with complexity and maintainability scores
- File analysis with language breakdown
- Dependency graph metrics
- LLM-generated insights and recommendations

### 🔍 Key Insights
- Architecture patterns detected
- Code quality assessment
- Security vulnerabilities (when enabled)
- Refactoring opportunities
- Documentation gaps

## Examples

You'll find examples of what reports are produced in the [./example-output/](./example-output/) directory.

### Analyze a React project
```bash
project-examer analyze --path ./my-react-app --output ./analysis-results
```

### First Time Setup
```bash
# Create configuration file with all options documented
project-examer config

# Edit the configuration file
vim ~/.project-examer.toml  # On Unix-like systems
notepad %USERPROFILE%\.project-examer.toml  # On Windows

# Set your API key (or use environment variable)
export OPENAI_API_KEY="your-openai-api-key"
```

### Analyze with custom LLM settings
```bash
# Using Ollama locally
project-examer config --output ollama-config.toml
# Edit ollama-config.toml to set provider="Ollama", base_url="http://localhost:11434"
project-examer analyze --config ollama-config.toml
```

### Local-only analysis (no API calls)
```bash
project-examer analyze --skip-llm
```

## Architecture

- **File Discovery**: Uses `ignore` crate for efficient file traversal
- **AST Parsing**: Tree-sitter for robust language parsing
- **Dependency Graph**: Petgraph for relationship modeling
- **Parallel Processing**: Rayon for multi-threaded file processing
- **LLM Integration**: Supports multiple providers with rate limiting

## Development

```bash
# Build
cargo build

# Run tests
cargo test

# Run with debug output
RUST_LOG=debug cargo run -- analyze --path ./test-project

# Build release version
cargo build --release
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Submit a pull request

## Use Cases

- **Code Reviews**: Understand codebase architecture before reviewing
- **Documentation**: Generate architectural documentation automatically
- **Refactoring**: Identify code smells and improvement opportunities
- **Security Audits**: Detect potential security vulnerabilities
- **Onboarding**: Help new team members understand project structure

## Performance

- Processes ~1000 files per second (varies by file size and complexity)
- Memory efficient with streaming file processing
- Parallel parsing for optimal CPU utilization
- Configurable rate limiting for LLM API calls

## License

MIT License - see LICENSE file for details.

---

**Project Examer** - Making codebase analysis fast, comprehensive, and intelligent. 🚀
