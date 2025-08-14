use crate::{
    config::Config,
    dependency_graph::{DependencyGraph, GraphBuilder},
    file_discovery::{FileDiscovery, FileInfo},
    llm::{AnalysisRequest, AnalysisContext, AnalysisType, FileContext, DependencyContext, ProjectInfo, LLMClient, AnalysisResponse},
    simple_parser::{SimpleParser, ParsedFile},
};
use anyhow::Result;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub struct Analyzer {
    config: Config,
    file_discovery: FileDiscovery,
    llm_client: LLMClient,
}

impl Analyzer {
    pub fn new(config: Config) -> Result<Self> {
        let file_discovery = FileDiscovery::new(config.clone());
        let llm_client = LLMClient::new(config.llm.clone());

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
        let context = self.create_analysis_context(parsed_files, _graph, files);
        
        let analysis_types = vec![
            AnalysisType::Overview,
            AnalysisType::Architecture,
            AnalysisType::Dependencies,
        ];

        let mut requests = Vec::new();
        for analysis_type in analysis_types {
            let prompt = self.create_prompt_for_type(&analysis_type);
            requests.push(AnalysisRequest {
                prompt,
                context: context.clone(),
                analysis_type,
            });
        }

        self.llm_client.batch_analyze(requests).await
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

        AnalysisContext {
            files: file_contexts,
            dependencies: dependency_contexts,
            project_info,
        }
    }

    fn create_prompt_for_type(&self, analysis_type: &AnalysisType) -> String {
        match analysis_type {
            AnalysisType::Overview => {
                "Provide a comprehensive overview of this software project. Describe what the software does, its main components, architecture style, and how different parts work together.".to_string()
            }
            AnalysisType::Architecture => {
                "Analyze the software architecture of this project. Identify architectural patterns (MVC, microservices, layered, etc.), design principles used, and the overall structural organization.".to_string()
            }
            AnalysisType::Dependencies => {
                "Analyze the dependency relationships in this codebase. Identify coupling issues, circular dependencies, and suggest improvements for better modularity.".to_string()
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