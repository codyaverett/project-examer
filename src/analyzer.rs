use crate::{
    config::Config,
    dependency_graph::{DependencyGraph, GraphBuilder},
    file_discovery::{FileDiscovery, FileInfo},
    llm::{AnalysisRequest, AnalysisContext, AnalysisType, FileContext, DependencyContext, ProjectInfo, LLMClient, AnalysisResponse, DocumentationContext},
    simple_parser::{SimpleParser, ParsedFile},
};
use anyhow::Result;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

pub struct Analyzer {
    config: Config,
    file_discovery: FileDiscovery,
    llm_client: LLMClient,
}

impl Analyzer {
    pub fn new(config: Config, debug_llm: bool) -> Result<Self> {
        let file_discovery = FileDiscovery::new(config.clone());
        let llm_client = LLMClient::new(config.llm.clone(), debug_llm);

        Ok(Self {
            config,
            file_discovery,
            llm_client,
        })
    }

    pub async fn analyze_project(&mut self, skip_llm: bool) -> Result<ProjectAnalysis> {
        println!("🔍 Discovering files...");
        let files = self.file_discovery.discover_files()?;
        let stats = self.file_discovery.get_stats(&files);
        stats.print_summary();

        println!("\n📝 Parsing files...");
        let parsed_files = self.parse_files_parallel(&files)?;

        println!("\n🕸️  Building dependency graph...");
        let mut graph_builder = GraphBuilder::new();
        let graph = graph_builder.build_graph(&parsed_files);
        
        // Clone the graph and get analysis before using in async function
        let graph_copy = graph.clone();
        let graph_analysis = graph_builder.analyze_dependencies();
        graph_analysis.print_summary();

        let llm_analysis = if skip_llm {
            println!("\n⚡ Skipping LLM analysis (local-only mode)");
            Vec::new()
        } else {
            println!("\n🤖 Analyzing with LLM...");
            self.analyze_with_llm(&parsed_files, &graph_copy, &files).await?
        };

        Ok(ProjectAnalysis {
            files: files.clone(),
            parsed_files,
            dependency_analysis: graph_analysis,
            llm_analysis,
        })
    }

    fn parse_files_parallel(&mut self, files: &[FileInfo]) -> Result<Vec<ParsedFile>> {
        let chunk_size = std::cmp::max(1, files.len() / rayon::current_num_threads());
        
        Ok(files
            .par_chunks(chunk_size)
            .map(|chunk| {
                let local_parser = SimpleParser::new().unwrap();
                let mut parsed_files = Vec::new();
                
                for file_info in chunk {
                    match local_parser.parse_file(file_info) {
                        Ok(parsed_file) => {
                            println!("  ✓ {}", file_info.path.display());
                            parsed_files.push(parsed_file);
                        }
                        Err(e) => {
                            eprintln!("  ✗ {}: {}", file_info.path.display(), e);
                        }
                    }
                }
                
                parsed_files
            })
            .reduce(Vec::new, |mut acc, mut chunk| {
                acc.append(&mut chunk);
                acc
            }))
    }

    async fn analyze_with_llm(
        &self,
        parsed_files: &[ParsedFile],
        _graph: &DependencyGraph,
        files: &[FileInfo],
    ) -> Result<Vec<AnalysisResponse>> {
        println!("  📊 Preparing analysis context...");
        let context = self.create_analysis_context(parsed_files, _graph, files);
        
        let analysis_types = vec![
            ("Overview", AnalysisType::Overview),
            ("Architecture", AnalysisType::Architecture), 
            ("Dependencies", AnalysisType::Dependencies),
        ];

        println!("  🔄 Running {} analysis types...", analysis_types.len());
        
        let mut results = Vec::new();
        for (i, (name, analysis_type)) in analysis_types.iter().enumerate() {
            println!("  {} Analyzing {} ({}/{})...", 
                if i == 0 { "🚀" } else { "📈" }, 
                name, 
                i + 1, 
                analysis_types.len()
            );
            
            let prompt = self.create_prompt_for_type(analysis_type);
            let request = AnalysisRequest {
                prompt,
                context: context.clone(),
                analysis_type: analysis_type.clone(),
            };

            match self.llm_client.analyze(request).await {
                Ok(response) => {
                    println!("    ✅ {} analysis completed", name);
                    results.push(response);
                }
                Err(e) => {
                    println!("    ⚠️  {} analysis failed: {}", name, e);
                    // Continue with other analyses even if one fails
                    println!("    📝 Continuing with remaining analyses...");
                }
            }
        }

        if results.is_empty() {
            println!("  ⚠️  All LLM analyses failed, continuing with local analysis only");
        } else {
            println!("  ✅ Completed {}/{} LLM analyses successfully", results.len(), analysis_types.len());
        }

        Ok(results)
    }

    fn create_analysis_context(
        &self,
        parsed_files: &[ParsedFile],
        _graph: &DependencyGraph,
        files: &[FileInfo],
    ) -> AnalysisContext {
        let file_contexts: Vec<FileContext> = parsed_files.iter().map(|pf| {
            FileContext {
                path: pf.file_info.path.to_string_lossy().to_string(),
                language: pf.file_info.language.clone().unwrap_or_else(|| "unknown".to_string()),
                content_summary: format!("{} functions, {} classes, {} imports", 
                    pf.functions.len(), pf.classes.len(), pf.imports.len()),
                functions: pf.functions.iter().map(|f| f.name.clone()).collect(),
                classes: pf.classes.iter().map(|c| c.name.clone()).collect(),
                imports: pf.imports.iter().map(|i| i.module.clone()).collect(),
            }
        }).collect();

        let dependency_contexts: Vec<DependencyContext> = parsed_files.iter().flat_map(|pf| {
            pf.imports.iter().map(|import| {
                DependencyContext {
                    from_file: pf.file_info.path.to_string_lossy().to_string(),
                    to_file: import.module.clone(),
                    dependency_type: "import".to_string(),
                    strength: 1.0,
                }
            })
        }).collect();

        let mut languages = HashMap::new();
        for file in files {
            if let Some(ref lang) = file.language {
                *languages.entry(lang.clone()).or_insert(0) += 1;
            }
        }

        let project_info = ProjectInfo {
            name: self.config.target_directory
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string(),
            total_files: files.len(),
            total_lines: files.iter().map(|f| f.size as usize).sum::<usize>() / 50, // Rough estimate
            languages: languages.keys().cloned().collect(),
            architecture_patterns: Vec::new(), // Will be filled by analysis
        };

        let documentation = self.extract_documentation_content(files);

        AnalysisContext {
            files: file_contexts,
            dependencies: dependency_contexts,
            project_info,
            documentation,
        }
    }

    fn safe_truncate<'a>(&self, s: &'a str, max_chars: usize) -> &'a str {
        if s.chars().count() <= max_chars {
            return s;
        }
        
        let mut end_idx = 0;
        for (i, (idx, _)) in s.char_indices().enumerate() {
            if i >= max_chars {
                break;
            }
            end_idx = idx;
        }
        
        // Find the next character boundary
        if let Some((next_idx, _)) = s.char_indices().nth(max_chars) {
            &s[..next_idx]
        } else {
            &s[..end_idx + s.chars().nth(max_chars.saturating_sub(1)).map_or(1, |c| c.len_utf8())]
        }
    }

    fn extract_documentation_content(&self, files: &[FileInfo]) -> Vec<DocumentationContext> {
        let mut documentation = Vec::new();
        
        for file in files {
            if let Some(ref language) = file.language {
                let is_documentation = matches!(language.as_str(), 
                    "markdown" | "text" | "json" | "yaml" | "toml");
                
                if is_documentation {
                    match fs::read_to_string(&file.path) {
                        Ok(content) => {
                            let summary = if content.chars().count() > 500 {
                                format!("{}... ({} characters total)", 
                                    self.safe_truncate(&content, 500), content.chars().count())
                            } else {
                                content.clone()
                            };
                            
                            documentation.push(DocumentationContext {
                                path: file.path.to_string_lossy().to_string(),
                                file_type: language.clone(),
                                content: if content.chars().count() > 8000 {
                                    // Truncate very long files but keep first and last parts
                                    let start_part = self.safe_truncate(&content, 4000);
                                    let total_chars = content.chars().count();
                                    let end_start = total_chars.saturating_sub(2000);
                                    let end_part: String = content.chars().skip(end_start).collect();
                                    
                                    format!("{}...\n\n[FILE TRUNCATED - {} total characters]\n\n...{}", 
                                        start_part, total_chars, end_part)
                                } else {
                                    content
                                },
                                summary,
                            });
                        }
                        Err(e) => {
                            eprintln!("Warning: Could not read documentation file {}: {}", 
                                file.path.display(), e);
                        }
                    }
                }
            }
        }
        
        documentation
    }

    fn create_prompt_for_type(&self, analysis_type: &AnalysisType) -> String {
        match analysis_type {
            AnalysisType::Overview => {
                r#"Provide a comprehensive overview of this software project in the following JSON format:

```json
{
  "analysis": "Brief overview of what the software does and its main purpose in 2-3 sentences",
  "insights": [
    {
      "title": "Key Insight Title",
      "description": "Detailed description of a key aspect, component, or characteristic of the project",
      "category": "Architecture|Functionality|Technology|Implementation",
      "confidence": 0.8,
      "evidence": [
        "Specific evidence from the codebase supporting this insight",
        "Another piece of evidence"
      ]
    }
  ],
  "recommendations": [
    {
      "title": "Recommendation Title",
      "description": "Detailed description of how to improve the project",
      "priority": "High|Medium|Low",
      "effort": "High|Medium|Low", 
      "impact": "High|Medium|Low",
      "action_items": [
        "Specific actionable step",
        "Another specific step"
      ]
    }
  ],
  "confidence": 0.8
}
```

Focus on describing what the software does, its main components, technology choices, architecture style, and how different parts work together. Use the provided documentation files (README, configuration files, etc.) to understand the project's purpose, goals, and design decisions."#.to_string()
            }
            AnalysisType::Architecture => {
                r#"Analyze the software architecture of this project and provide insights in the following JSON format:

```json
{
  "analysis": "Brief architectural overview of the project in 2-3 sentences",
  "insights": [
    {
      "title": "Architecture Pattern Name",
      "description": "Detailed description of the architectural pattern or design principle identified",
      "category": "Architecture|Design Pattern|Structure|Organization",
      "confidence": 0.8,
      "evidence": [
        "Specific evidence from the codebase supporting this insight",
        "Another piece of evidence"
      ]
    }
  ],
  "recommendations": [
    {
      "title": "Recommendation Title",
      "description": "Detailed description of the architectural improvement",
      "priority": "High|Medium|Low",
      "effort": "High|Medium|Low", 
      "impact": "High|Medium|Low",
      "action_items": [
        "Specific actionable step",
        "Another specific step"
      ]
    }
  ],
  "confidence": 0.8
}
```

Focus on identifying architectural patterns (MVC, microservices, layered, etc.), design principles (SOLID, DRY, etc.), structural organization, modularity, and provide actionable recommendations for architectural improvements. Use the provided documentation to understand the intended architecture and design decisions."#.to_string()
            }
            AnalysisType::Dependencies => {
                r#"Analyze the dependency relationships in this codebase and provide insights in the following JSON format:

```json
{
  "analysis": "Brief summary of the dependency structure and key findings in 2-3 sentences",
  "insights": [
    {
      "title": "Dependency Issue or Pattern Name",
      "description": "Detailed description of the dependency pattern, coupling issue, or modularity aspect identified",
      "category": "Coupling|Modularity|Dependencies|Structure",
      "confidence": 0.8,
      "evidence": [
        "Specific evidence from the codebase supporting this insight",
        "Another piece of evidence"
      ]
    }
  ],
  "recommendations": [
    {
      "title": "Recommendation Title",
      "description": "Detailed description of how to improve dependency management or modularity",
      "priority": "High|Medium|Low",
      "effort": "High|Medium|Low", 
      "impact": "High|Medium|Low",
      "action_items": [
        "Specific actionable step to improve dependencies",
        "Another specific step"
      ]
    }
  ],
  "confidence": 0.8
}
```

Focus on identifying coupling issues, circular dependencies, modularity problems, dependency injection opportunities, and provide actionable recommendations for better dependency management. Consider the project's documentation to understand intended module relationships and design goals."#.to_string()
            }
            AnalysisType::Security => {
                "Perform a security analysis of this codebase. Look for potential vulnerabilities, insecure patterns, and provide security recommendations.".to_string()
            }
            AnalysisType::Refactoring => {
                "Identify refactoring opportunities in this codebase. Look for code smells, duplication, and areas that could benefit from restructuring.".to_string()
            }
            AnalysisType::Documentation => {
                "Generate comprehensive documentation for this software project, explaining how it works, its components, and usage patterns.".to_string()
            }
        }
    }

    pub fn get_file_summary(&self, files: &[FileInfo]) -> FileSummary {
        let mut summary = FileSummary::default();
        
        for file in files {
            summary.total_files += 1;
            summary.total_size += file.size;
            
            if let Some(ref lang) = file.language {
                *summary.language_distribution.entry(lang.clone()).or_insert(0) += 1;
            }
            
            if let Some(ref ext) = file.extension {
                *summary.extension_distribution.entry(ext.clone()).or_insert(0) += 1;
            }
        }
        
        summary
    }

    pub fn filter_files_by_criteria<'a>(&self, files: &'a [FileInfo], criteria: &FilterCriteria) -> Vec<&'a FileInfo> {
        files.iter().filter(|file| {
            if let Some(ref lang_filter) = criteria.language {
                if file.language.as_ref() != Some(lang_filter) {
                    return false;
                }
            }
            
            if let Some(min_size) = criteria.min_size {
                if file.size < min_size {
                    return false;
                }
            }
            
            if let Some(max_size) = criteria.max_size {
                if file.size > max_size {
                    return false;
                }
            }
            
            if let Some(ref path_contains) = criteria.path_contains {
                if !file.path.to_string_lossy().contains(path_contains) {
                    return false;
                }
            }
            
            true
        }).collect()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectAnalysis {
    pub files: Vec<FileInfo>,
    pub parsed_files: Vec<ParsedFile>,
    pub dependency_analysis: crate::dependency_graph::DependencyAnalysis,
    pub llm_analysis: Vec<AnalysisResponse>,
}

impl ProjectAnalysis {
    pub fn print_summary(&self) {
        println!("📊 Project Analysis Summary");
        println!("==========================");
        
        println!("\n📁 Files:");
        println!("  Total files: {}", self.files.len());
        println!("  Successfully parsed: {}", self.parsed_files.len());
        
        println!("\n🔗 Dependencies:");
        self.dependency_analysis.print_summary();
        
        println!("\n🤖 LLM Analysis:");
        for (i, analysis) in self.llm_analysis.iter().enumerate() {
            println!("  Analysis {}:", i + 1);
            println!("    Confidence: {:.2}", analysis.confidence);
            println!("    Insights: {}", analysis.insights.len());
            println!("    Recommendations: {}", analysis.recommendations.len());
        }
    }

    pub fn export_to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct FileSummary {
    pub total_files: usize,
    pub total_size: u64,
    pub language_distribution: HashMap<String, usize>,
    pub extension_distribution: HashMap<String, usize>,
}

#[derive(Debug, Default)]
pub struct FilterCriteria {
    pub language: Option<String>,
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,
    pub path_contains: Option<String>,
}