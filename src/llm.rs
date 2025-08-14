use crate::config::{LLMConfig, LLMProvider};
use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize)]
pub struct AnalysisRequest {
    pub prompt: String,
    pub context: AnalysisContext,
    pub analysis_type: AnalysisType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisContext {
    pub files: Vec<FileContext>,
    pub dependencies: Vec<DependencyContext>,
    pub project_info: ProjectInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileContext {
    pub path: String,
    pub language: String,
    pub content_summary: String,
    pub functions: Vec<String>,
    pub classes: Vec<String>,
    pub imports: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyContext {
    pub from_file: String,
    pub to_file: String,
    pub dependency_type: String,
    pub strength: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub name: String,
    pub total_files: usize,
    pub total_lines: usize,
    pub languages: Vec<String>,
    pub architecture_patterns: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum AnalysisType {
    Overview,
    Architecture,
    Dependencies,
    Security,
    Refactoring,
    Documentation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResponse {
    pub analysis: String,
    pub insights: Vec<Insight>,
    pub recommendations: Vec<Recommendation>,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Insight {
    pub title: String,
    pub description: String,
    pub category: InsightCategory,
    pub confidence: f64,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InsightCategory {
    Architecture,
    CodeQuality,
    Performance,
    Security,
    Maintainability,
    Testing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    pub title: String,
    pub description: String,
    pub priority: Priority,
    pub effort: Effort,
    pub impact: Impact,
    pub action_items: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Effort {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Impact {
    Low,
    Medium,
    High,
}

pub struct LLMClient {
    config: LLMConfig,
    client: Client,
}

impl LLMClient {
    pub fn new(config: LLMConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .unwrap();

        Self { config, client }
    }

    pub async fn analyze(&self, request: AnalysisRequest) -> Result<AnalysisResponse> {
        match self.config.provider {
            LLMProvider::OpenAI => self.analyze_with_openai(request).await,
            LLMProvider::Ollama => self.analyze_with_ollama(request).await,
            LLMProvider::Anthropic => self.analyze_with_anthropic(request).await,
        }
    }

    async fn analyze_with_openai(&self, request: AnalysisRequest) -> Result<AnalysisResponse> {
        let api_key = self.config.api_key.as_ref()
            .ok_or_else(|| anyhow!("OpenAI API key not provided"))?;

        let system_prompt = self.create_system_prompt(&request.analysis_type);
        let user_prompt = self.create_user_prompt(&request);

        let payload = serde_json::json!({
            "model": self.config.model,
            "messages": [
                {
                    "role": "system",
                    "content": system_prompt
                },
                {
                    "role": "user",
                    "content": user_prompt
                }
            ],
            "max_tokens": self.config.max_tokens,
            "temperature": self.config.temperature,
            "response_format": {
                "type": "json_object"
            }
        });

        let response = self.client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("OpenAI API error: {}", error_text));
        }

        let response_json: serde_json::Value = response.json().await?;
        let content = response_json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| anyhow!("Invalid response format from OpenAI"))?;

        let analysis_response: AnalysisResponse = serde_json::from_str(content)?;
        Ok(analysis_response)
    }

    async fn analyze_with_ollama(&self, request: AnalysisRequest) -> Result<AnalysisResponse> {
        let default_url = "http://localhost:11434".to_string();
        let base_url = self.config.base_url.as_ref().unwrap_or(&default_url);

        let system_prompt = self.create_system_prompt(&request.analysis_type);
        let user_prompt = self.create_user_prompt(&request);

        let payload = serde_json::json!({
            "model": self.config.model,
            "messages": [
                {
                    "role": "system",
                    "content": system_prompt
                },
                {
                    "role": "user", 
                    "content": user_prompt
                }
            ],
            "stream": false,
            "format": "json",
            "options": {
                "temperature": self.config.temperature,
                "num_predict": self.config.max_tokens
            }
        });

        let response = self.client
            .post(&format!("{}/api/chat", base_url))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("Ollama API error: {}", error_text));
        }

        let response_json: serde_json::Value = response.json().await?;
        let content = response_json["message"]["content"]
            .as_str()
            .ok_or_else(|| anyhow!("Invalid response format from Ollama"))?;

        let analysis_response: AnalysisResponse = serde_json::from_str(content)?;
        Ok(analysis_response)
    }

    async fn analyze_with_anthropic(&self, request: AnalysisRequest) -> Result<AnalysisResponse> {
        let api_key = self.config.api_key.as_ref()
            .ok_or_else(|| anyhow!("Anthropic API key not provided"))?;

        let system_prompt = self.create_system_prompt(&request.analysis_type);
        let user_prompt = self.create_user_prompt(&request);

        let payload = serde_json::json!({
            "model": self.config.model,
            "max_tokens": self.config.max_tokens,
            "system": system_prompt,
            "messages": [
                {
                    "role": "user",
                    "content": user_prompt
                }
            ]
        });

        let response = self.client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("Content-Type", "application/json")
            .header("anthropic-version", "2023-06-01")
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("Anthropic API error: {}", error_text));
        }

        let response_json: serde_json::Value = response.json().await?;
        let content = response_json["content"][0]["text"]
            .as_str()
            .ok_or_else(|| anyhow!("Invalid response format from Anthropic"))?;

        let analysis_response: AnalysisResponse = serde_json::from_str(content)?;
        Ok(analysis_response)
    }

    fn create_system_prompt(&self, analysis_type: &AnalysisType) -> String {
        match analysis_type {
            AnalysisType::Overview => {
                "You are a senior software architect analyzing a codebase. Provide a comprehensive overview of the software architecture, including key components, patterns used, and overall design philosophy. Return your response as JSON with the following structure: {\"analysis\": \"...\", \"insights\": [...], \"recommendations\": [...], \"confidence\": 0.0-1.0}".to_string()
            }
            AnalysisType::Architecture => {
                "You are a software architect expert. Analyze the architectural patterns, design principles, and structural organization of this codebase. Identify patterns like MVC, microservices, layered architecture, etc. Return your response as JSON.".to_string()
            }
            AnalysisType::Dependencies => {
                "You are a dependency analysis expert. Examine the dependency relationships, identify potential issues like circular dependencies, tight coupling, or unused dependencies. Return your response as JSON.".to_string()
            }
            AnalysisType::Security => {
                "You are a security expert analyzing code for potential vulnerabilities. Look for common security issues, insecure patterns, and provide recommendations for improvement. Return your response as JSON.".to_string()
            }
            AnalysisType::Refactoring => {
                "You are a code quality expert. Identify opportunities for refactoring, code smells, and suggest improvements for maintainability and readability. Return your response as JSON.".to_string()
            }
            AnalysisType::Documentation => {
                "You are a technical documentation expert. Generate comprehensive documentation based on the code structure and patterns. Create explanations for how the software works. Return your response as JSON.".to_string()
            }
        }
    }

    fn create_user_prompt(&self, request: &AnalysisRequest) -> String {
        let mut prompt = format!("Analyze this codebase:\n\n{}\n\n", request.prompt);

        prompt.push_str("Project Information:\n");
        prompt.push_str(&format!("- Name: {}\n", request.context.project_info.name));
        prompt.push_str(&format!("- Total files: {}\n", request.context.project_info.total_files));
        prompt.push_str(&format!("- Languages: {}\n", request.context.project_info.languages.join(", ")));

        if !request.context.files.is_empty() {
            prompt.push_str("\nFile Structure:\n");
            for file in &request.context.files {
                prompt.push_str(&format!("- {} ({})\n", file.path, file.language));
                prompt.push_str(&format!("  Functions: {}\n", file.functions.join(", ")));
                if !file.classes.is_empty() {
                    prompt.push_str(&format!("  Classes: {}\n", file.classes.join(", ")));
                }
                if !file.imports.is_empty() {
                    prompt.push_str(&format!("  Imports: {}\n", file.imports.join(", ")));
                }
            }
        }

        if !request.context.dependencies.is_empty() {
            prompt.push_str("\nDependency Relationships:\n");
            for dep in &request.context.dependencies {
                prompt.push_str(&format!("- {} -> {} ({}, strength: {:.2})\n", 
                    dep.from_file, dep.to_file, dep.dependency_type, dep.strength));
            }
        }

        prompt.push_str("\nPlease provide a detailed analysis with specific insights and actionable recommendations.");
        prompt
    }

    pub async fn batch_analyze(&self, requests: Vec<AnalysisRequest>) -> Result<Vec<AnalysisResponse>> {
        let mut responses = Vec::new();
        
        for request in requests {
            let response = self.analyze(request).await?;
            responses.push(response);
            
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        
        Ok(responses)
    }
}