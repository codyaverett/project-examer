use crate::{
    analyzer::{ProjectAnalysis, FileSummary},
    dependency_graph::DependencyAnalysis,
    llm::{AnalysisResponse, Priority},
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::PathBuf,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Report {
    pub metadata: ReportMetadata,
    pub executive_summary: ExecutiveSummary,
    pub file_analysis: FileAnalysisReport,
    pub dependency_analysis: DependencyAnalysisReport,
    pub llm_insights: Vec<AnalysisResponse>,
    pub recommendations: Vec<PrioritizedRecommendation>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReportMetadata {
    pub generated_at: String,
    pub project_name: String,
    pub total_files: usize,
    pub total_size: u64,
    pub analysis_duration_ms: u128,
    pub version: String,
    pub llm_provider: String,
    pub llm_model: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutiveSummary {
    pub overview: String,
    pub key_findings: Vec<String>,
    pub critical_issues: Vec<String>,
    pub architecture_style: String,
    pub complexity_score: f64,
    pub maintainability_score: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileAnalysisReport {
    pub summary: FileSummary,
    pub language_breakdown: Vec<LanguageStats>,
    pub largest_files: Vec<FileStats>,
    pub complexity_distribution: Vec<ComplexityBucket>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LanguageStats {
    pub language: String,
    pub file_count: usize,
    pub total_size: u64,
    pub avg_file_size: f64,
    pub percentage: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileStats {
    pub path: String,
    pub size: u64,
    pub language: String,
    pub functions: usize,
    pub classes: usize,
    pub complexity: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ComplexityBucket {
    pub range: String,
    pub count: usize,
    pub percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyAnalysisReport {
    pub graph_metrics: DependencyAnalysis,
    pub circular_dependencies: Vec<CircularDependency>,
    pub highly_coupled_files: Vec<CouplingInfo>,
    pub orphaned_files: Vec<String>,
    pub dependency_depth: DependencyDepthInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircularDependency {
    pub files: Vec<String>,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CouplingInfo {
    pub file: String,
    pub incoming_dependencies: usize,
    pub outgoing_dependencies: usize,
    pub coupling_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyDepthInfo {
    pub max_depth: usize,
    pub avg_depth: f64,
    pub depth_distribution: Vec<DepthBucket>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepthBucket {
    pub depth: usize,
    pub count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PrioritizedRecommendation {
    pub title: String,
    pub description: String,
    pub priority: Priority,
    pub category: String,
    pub estimated_effort: String,
    pub potential_impact: String,
    pub action_items: Vec<String>,
    pub affected_files: Vec<String>,
}

pub struct Reporter;

impl Reporter {
    pub fn new() -> Self {
        Self
    }

    pub fn generate_report(&self, analysis: &ProjectAnalysis, duration_ms: u128, llm_provider: &str, llm_model: &str) -> Report {
        let metadata = self.create_metadata(analysis, duration_ms, llm_provider, llm_model);
        let executive_summary = self.create_executive_summary(analysis);
        let file_analysis = self.create_file_analysis_report(analysis);
        let dependency_analysis = self.create_dependency_analysis_report(analysis);
        let recommendations = self.prioritize_recommendations(analysis);

        Report {
            metadata,
            executive_summary,
            file_analysis,
            dependency_analysis,
            llm_insights: analysis.llm_analysis.clone(),
            recommendations,
        }
    }

    fn create_metadata(&self, analysis: &ProjectAnalysis, duration_ms: u128, llm_provider: &str, llm_model: &str) -> ReportMetadata {
        let total_size = analysis.files.iter().map(|f| f.size).sum();
        let project_name = analysis.files.first()
            .and_then(|f| f.path.parent())
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        ReportMetadata {
            generated_at: chrono::Utc::now().to_rfc3339(),
            project_name,
            total_files: analysis.files.len(),
            total_size,
            analysis_duration_ms: duration_ms,
            version: env!("CARGO_PKG_VERSION").to_string(),
            llm_provider: llm_provider.to_string(),
            llm_model: llm_model.to_string(),
        }
    }

    fn create_executive_summary(&self, analysis: &ProjectAnalysis) -> ExecutiveSummary {
        let mut key_findings = Vec::new();
        let mut critical_issues = Vec::new();

        for analysis_result in &analysis.llm_analysis {
            for insight in &analysis_result.insights {
                key_findings.push(insight.title.clone());
            }

            for rec in &analysis_result.recommendations {
                if matches!(rec.priority, Priority::High | Priority::Critical) {
                    critical_issues.push(rec.title.clone());
                }
            }
        }

        let overview = if let Some(first_analysis) = analysis.llm_analysis.first() {
            first_analysis.analysis.clone()
        } else {
            "No LLM analysis available".to_string()
        };

        let complexity_score = self.calculate_complexity_score(analysis);
        let maintainability_score = self.calculate_maintainability_score(analysis);

        ExecutiveSummary {
            overview,
            key_findings,
            critical_issues,
            architecture_style: "Unknown".to_string(), // Could be inferred from analysis
            complexity_score,
            maintainability_score,
        }
    }

    fn create_file_analysis_report(&self, analysis: &ProjectAnalysis) -> FileAnalysisReport {
        let total_size: u64 = analysis.files.iter().map(|f| f.size).sum();
        
        let mut language_stats: std::collections::HashMap<String, (usize, u64)> = std::collections::HashMap::new();
        for file in &analysis.files {
            if let Some(ref lang) = file.language {
                let entry = language_stats.entry(lang.clone()).or_insert((0, 0));
                entry.0 += 1;
                entry.1 += file.size;
            }
        }

        let language_breakdown: Vec<LanguageStats> = language_stats
            .into_iter()
            .map(|(lang, (count, size))| LanguageStats {
                language: lang,
                file_count: count,
                total_size: size,
                avg_file_size: size as f64 / count as f64,
                percentage: (count as f64 / analysis.files.len() as f64) * 100.0,
            })
            .collect();

        let mut file_stats: Vec<FileStats> = analysis.parsed_files
            .iter()
            .map(|pf| FileStats {
                path: pf.file_info.path.to_string_lossy().to_string(),
                size: pf.file_info.size,
                language: pf.file_info.language.clone().unwrap_or_else(|| "unknown".to_string()),
                functions: pf.functions.len(),
                classes: pf.classes.len(),
                complexity: pf.functions.len() + pf.classes.len() * 2,
            })
            .collect();

        file_stats.sort_by(|a, b| b.size.cmp(&a.size));
        let largest_files = file_stats.into_iter().take(10).collect();

        let complexity_distribution = self.calculate_complexity_distribution(analysis);

        FileAnalysisReport {
            summary: FileSummary {
                total_files: analysis.files.len(),
                total_size,
                language_distribution: std::collections::HashMap::new(),
                extension_distribution: std::collections::HashMap::new(),
            },
            language_breakdown,
            largest_files,
            complexity_distribution,
        }
    }

    fn create_dependency_analysis_report(&self, analysis: &ProjectAnalysis) -> DependencyAnalysisReport {
        DependencyAnalysisReport {
            graph_metrics: analysis.dependency_analysis.clone(),
            circular_dependencies: Vec::new(), // TODO: Implement circular dependency detection
            highly_coupled_files: Vec::new(),   // TODO: Implement coupling analysis
            orphaned_files: Vec::new(),         // TODO: Implement orphan detection
            dependency_depth: DependencyDepthInfo {
                max_depth: 0,
                avg_depth: 0.0,
                depth_distribution: Vec::new(),
            },
        }
    }

    fn prioritize_recommendations(&self, analysis: &ProjectAnalysis) -> Vec<PrioritizedRecommendation> {
        let mut recommendations = Vec::new();

        for analysis_result in &analysis.llm_analysis {
            for rec in &analysis_result.recommendations {
                recommendations.push(PrioritizedRecommendation {
                    title: rec.title.clone(),
                    description: rec.description.clone(),
                    priority: rec.priority.clone(),
                    category: "General".to_string(),
                    estimated_effort: format!("{:?}", rec.effort),
                    potential_impact: format!("{:?}", rec.impact),
                    action_items: rec.action_items.clone(),
                    affected_files: Vec::new(),
                });
            }
        }

        recommendations.sort_by(|a, b| {
            use Priority::*;
            let priority_order = |p: &Priority| match p {
                Critical => 0,
                High => 1,
                Medium => 2,
                Low => 3,
            };
            priority_order(&a.priority).cmp(&priority_order(&b.priority))
        });

        recommendations
    }

    fn calculate_complexity_score(&self, analysis: &ProjectAnalysis) -> f64 {
        if analysis.parsed_files.is_empty() {
            return 0.0;
        }

        let total_complexity: usize = analysis.parsed_files
            .iter()
            .map(|pf| pf.functions.len() + pf.classes.len() * 2 + pf.imports.len())
            .sum();

        (total_complexity as f64 / analysis.parsed_files.len() as f64).min(10.0)
    }

    fn calculate_maintainability_score(&self, analysis: &ProjectAnalysis) -> f64 {
        let complexity = self.calculate_complexity_score(analysis);
        let coupling = analysis.dependency_analysis.avg_degree;
        
        let base_score = 10.0;
        let complexity_penalty = complexity * 0.5;
        let coupling_penalty = coupling * 0.3;
        
        (base_score - complexity_penalty - coupling_penalty).max(0.0)
    }

    fn calculate_complexity_distribution(&self, analysis: &ProjectAnalysis) -> Vec<ComplexityBucket> {
        let mut buckets = vec![
            ComplexityBucket { range: "0-5".to_string(), count: 0, percentage: 0.0 },
            ComplexityBucket { range: "6-15".to_string(), count: 0, percentage: 0.0 },
            ComplexityBucket { range: "16-30".to_string(), count: 0, percentage: 0.0 },
            ComplexityBucket { range: "31+".to_string(), count: 0, percentage: 0.0 },
        ];

        for pf in &analysis.parsed_files {
            let complexity = pf.functions.len() + pf.classes.len() * 2;
            match complexity {
                0..=5 => buckets[0].count += 1,
                6..=15 => buckets[1].count += 1,
                16..=30 => buckets[2].count += 1,
                _ => buckets[3].count += 1,
            }
        }

        let total = analysis.parsed_files.len() as f64;
        for bucket in &mut buckets {
            bucket.percentage = (bucket.count as f64 / total) * 100.0;
        }

        buckets
    }

    pub fn export_report(&self, report: &Report, output_dir: &PathBuf) -> Result<Vec<PathBuf>> {
        fs::create_dir_all(output_dir)?;
        let mut exported_files = Vec::new();

        // Export JSON report
        let json_path = output_dir.join("analysis_report.json");
        let json_content = serde_json::to_string_pretty(report)?;
        fs::write(&json_path, json_content)?;
        exported_files.push(json_path);

        // Export HTML report
        let html_path = output_dir.join("analysis_report.html");
        let html_content = self.generate_html_report(report)?;
        fs::write(&html_path, html_content)?;
        exported_files.push(html_path);

        // Export Markdown summary
        let md_path = output_dir.join("analysis_summary.md");
        let md_content = self.generate_markdown_summary(report)?;
        fs::write(&md_path, md_content)?;
        exported_files.push(md_path);

        Ok(exported_files)
    }

    fn generate_html_report(&self, report: &Report) -> Result<String> {
        let html = format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Project Analysis Report - {}</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 40px; line-height: 1.6; }}
        .header {{ border-bottom: 2px solid #333; padding-bottom: 20px; }}
        .section {{ margin: 30px 0; }}
        .metric {{ display: inline-block; margin: 10px 20px 10px 0; padding: 10px; background: #f5f5f5; border-radius: 5px; }}
        .recommendation {{ margin: 15px 0; padding: 15px; border-left: 4px solid #007acc; background: #f9f9f9; }}
        .priority-high {{ border-left-color: #ff6b6b; }}
        .priority-medium {{ border-left-color: #ffa500; }}
        .priority-low {{ border-left-color: #28a745; }}
        .insight {{ margin: 10px 0; padding: 10px; background: #e8f4f8; border-radius: 5px; }}
        .insight-title {{ font-weight: bold; color: #2c3e50; }}
        .insight-category {{ color: #7f8c8d; font-size: 0.9em; text-transform: uppercase; }}
        .evidence {{ margin: 5px 0; font-style: italic; color: #555; }}
        .llm-analysis {{ margin: 20px 0; padding: 20px; background: #f8f9fa; border-radius: 8px; }}
        .analysis-type {{ font-weight: bold; color: #495057; margin-bottom: 10px; }}
        .analysis-summary {{ margin: 10px 0; padding: 15px; background: #fff; border-radius: 5px; line-height: 1.6; }}
        .insights-table, .recommendations-table {{ margin: 15px 0; }}
        .insights-table th {{ background-color: #e3f2fd; }}
        .recommendations-table th {{ background-color: #f3e5f5; }}
        table {{ border-collapse: collapse; width: 100%; margin: 10px 0; }}
        th, td {{ border: 1px solid #ddd; padding: 12px; text-align: left; vertical-align: top; }}
        th {{ background-color: #f2f2f2; font-weight: bold; }}
        .priority-high {{ background-color: #ffebee; }}
        .priority-medium {{ background-color: #fff3e0; }}
        .priority-low {{ background-color: #f1f8e9; }}
        .confidence-high {{ color: #2e7d32; font-weight: bold; }}
        .confidence-medium {{ color: #f57c00; font-weight: bold; }}
        .confidence-low {{ color: #d32f2f; font-weight: bold; }}
        ol {{ list-style-type: decimal; padding-left: 25px; margin: 10px 0; }}
        ul {{ list-style-type: disc; padding-left: 25px; margin: 10px 0; }}
        li {{ margin: 8px 0; line-height: 1.4; }}
        .analysis-summary ul {{ margin: 15px 0; }}
        .analysis-summary ol {{ margin: 15px 0; }}
        .analysis-summary li {{ margin: 6px 0; padding-left: 5px; }}
        .analysis-summary h4 {{ margin: 20px 0 10px 0; color: #2c3e50; }}
        .analysis-summary h3 {{ margin: 25px 0 15px 0; color: #34495e; }}
        .analysis-summary p {{ margin: 12px 0; line-height: 1.6; }}
    </style>
    <script>
        function parseJsonContent(jsonText) {{
            try {{
                const data = JSON.parse(jsonText);
                let html = '';
                
                // Analysis summary
                if (data.analysis) {{
                    html += `<div class="analysis-summary">${{data.analysis}}</div>`;
                }}
                
                // Insights table
                if (data.insights && data.insights.length > 0) {{
                    html += `
                    <h4>Key Insights</h4>
                    <table class="insights-table">
                        <thead>
                            <tr>
                                <th>Insight</th>
                                <th>Category</th>
                                <th>Description</th>
                                <th>Confidence</th>
                                <th>Evidence</th>
                            </tr>
                        </thead>
                        <tbody>`;
                    
                    data.insights.forEach(insight => {{
                        const confidenceClass = insight.confidence >= 0.8 ? 'confidence-high' : 
                                               insight.confidence >= 0.6 ? 'confidence-medium' : 'confidence-low';
                        const evidence = insight.evidence && insight.evidence.length > 0 ? 
                                        '• ' + insight.evidence.join('<br>• ') : 'No specific evidence';
                        
                        html += `
                        <tr>
                            <td><strong>${{insight.title}}</strong></td>
                            <td>${{insight.category}}</td>
                            <td>${{insight.description}}</td>
                            <td class="${{confidenceClass}}">${{Math.round(insight.confidence * 100)}}%</td>
                            <td>${{evidence}}</td>
                        </tr>`;
                    }});
                    
                    html += '</tbody></table>';
                }}
                
                // Recommendations table
                if (data.recommendations && data.recommendations.length > 0) {{
                    html += `
                    <h4>Recommendations</h4>
                    <table class="recommendations-table">
                        <thead>
                            <tr>
                                <th>Title</th>
                                <th>Description</th>
                                <th>Priority</th>
                                <th>Effort</th>
                                <th>Impact</th>
                                <th>Action Items</th>
                            </tr>
                        </thead>
                        <tbody>`;
                    
                    data.recommendations.forEach(rec => {{
                        const priorityClass = rec.priority === 'High' || rec.priority === 'Critical' ? 'priority-high' :
                                             rec.priority === 'Medium' ? 'priority-medium' : 'priority-low';
                        const actionItems = rec.action_items && rec.action_items.length > 0 ? 
                                           '• ' + rec.action_items.join('<br>• ') : 'No specific actions';
                        
                        html += `
                        <tr class="${{priorityClass}}">
                            <td><strong>${{rec.title}}</strong></td>
                            <td>${{rec.description}}</td>
                            <td>${{rec.priority}}</td>
                            <td>${{rec.effort}}</td>
                            <td>${{rec.impact}}</td>
                            <td>${{actionItems}}</td>
                        </tr>`;
                    }});
                    
                    html += '</tbody></table>';
                }}
                
                return html;
            }} catch (e) {{
                return `<p>Error parsing JSON content: ${{e.message}}</p>`;
            }}
        }}
        
        function parseMarkdownContent(markdown) {{
            let html = markdown;
            
            // Convert headers first
            html = html.replace(/^#### (.+)$/gm, '<h4>$1</h4>');
            html = html.replace(/^### (.+)$/gm, '<h3>$1</h3>');
            html = html.replace(/^## (.+)$/gm, '<h2>$1</h2>');
            html = html.replace(/^# (.+)$/gm, '<h1>$1</h1>');
            
            // Convert bold text
            html = html.replace(/\*\*(.*?)\*\*/g, '<strong>$1</strong>');
            
            // Process line by line for better list handling
            let lines = html.split('\n');
            let processedLines = [];
            let inUnorderedList = false;
            let inOrderedList = false;
            
            for (let i = 0; i < lines.length; i++) {{
                let line = lines[i];
                let trimmedLine = line.trim();
                
                // Look ahead to see if there are more list items coming
                function hasMoreListItems(startIndex, listType) {{
                    for (let j = startIndex + 1; j < lines.length; j++) {{
                        let nextTrimmed = lines[j].trim();
                        if (nextTrimmed === '') continue; // Skip empty lines
                        
                        if (listType === 'ordered' && nextTrimmed.match(/^\d+\.\s+/)) {{
                            return true;
                        }}
                        if (listType === 'unordered' && nextTrimmed.match(/^[-*]\s+/)) {{
                            return true;
                        }}
                        
                        // Stop looking if we hit a header or substantial content
                        if (nextTrimmed.match(/^<h[1-6]>/) || 
                            nextTrimmed.match(/^### /) || 
                            nextTrimmed.match(/^## /) ||
                            nextTrimmed.match(/^#### /) ||
                            (nextTrimmed.length > 0 && !nextTrimmed.match(/^[-*\d]\s*/) && !nextTrimmed.match(/^\d+\.\s+/))) {{
                            break;
                        }}
                    }}
                    return false;
                }}
                
                // Handle unordered list items
                if (trimmedLine.match(/^[-*]\s+/)) {{
                    if (!inUnorderedList) {{
                        if (inOrderedList) {{
                            processedLines.push('</ol>');
                            inOrderedList = false;
                        }}
                        processedLines.push('<ul>');
                        inUnorderedList = true;
                    }}
                    let content = trimmedLine.replace(/^[-*]\s+/, '');
                    processedLines.push(`<li>${{content}}</li>`);
                    
                    // Only close if no more unordered items are coming
                    if (!hasMoreListItems(i, 'unordered')) {{
                        processedLines.push('</ul>');
                        inUnorderedList = false;
                    }}
                }}
                // Handle ordered list items (1. 2. 3. etc.)
                else if (trimmedLine.match(/^\d+\.\s+/)) {{
                    if (!inOrderedList) {{
                        if (inUnorderedList) {{
                            processedLines.push('</ul>');
                            inUnorderedList = false;
                        }}
                        processedLines.push('<ol>');
                        inOrderedList = true;
                    }}
                    let content = trimmedLine.replace(/^\d+\.\s+/, '');
                    processedLines.push(`<li>${{content}}</li>`);
                    
                    // Only close if no more ordered items are coming
                    if (!hasMoreListItems(i, 'ordered')) {{
                        processedLines.push('</ol>');
                        inOrderedList = false;
                    }}
                }}
                // Handle regular content
                else {{
                    // Close lists when we encounter headers or substantial content
                    if (trimmedLine && (trimmedLine.startsWith('<h') || 
                        trimmedLine.match(/^### /) || 
                        trimmedLine.match(/^## /) ||
                        trimmedLine.match(/^#### /))) {{
                        // Close any open lists when we hit headers
                        if (inUnorderedList) {{
                            processedLines.push('</ul>');
                            inUnorderedList = false;
                        }}
                        if (inOrderedList) {{
                            processedLines.push('</ol>');
                            inOrderedList = false;
                        }}
                        processedLines.push(line);
                    }} else if (trimmedLine && !trimmedLine.startsWith('<ul') && !trimmedLine.startsWith('<ol') && !trimmedLine.startsWith('</')) {{
                        // Close lists for substantial paragraph content
                        if (inUnorderedList) {{
                            processedLines.push('</ul>');
                            inUnorderedList = false;
                        }}
                        if (inOrderedList) {{
                            processedLines.push('</ol>');
                            inOrderedList = false;
                        }}
                        processedLines.push(`<p>${{line}}</p>`);
                    }} else {{
                        // Empty lines and HTML elements - keep them without closing lists
                        processedLines.push(line);
                    }}
                }}
            }}
            
            // Close any remaining open lists
            if (inUnorderedList) {{
                processedLines.push('</ul>');
            }}
            if (inOrderedList) {{
                processedLines.push('</ol>');
            }}
            
            return processedLines.join('\n');
        }}
        
        document.addEventListener('DOMContentLoaded', function() {{
            // Process JSON content in any element that contains JSON
            function processElementForJson(element) {{
                const text = element.textContent || element.innerText;
                if (text.trim().startsWith('```json') && text.trim().endsWith('```')) {{
                    const jsonContent = text.trim().slice(7, -3); // Remove ```json and ```
                    const processedHtml = parseJsonContent(jsonContent);
                    element.innerHTML = processedHtml;
                    element.style.whiteSpace = 'normal';
                    return true;
                }} else if (text.trim().startsWith('{{') && text.trim().endsWith('}}')) {{
                    // Direct JSON content without markdown code blocks
                    try {{
                        const processedHtml = parseJsonContent(text.trim());
                        element.innerHTML = processedHtml;
                        element.style.whiteSpace = 'normal';
                        return true;
                    }} catch (e) {{
                        // Not valid JSON, continue to markdown processing
                    }}
                }}
                return false;
            }}
            
            // Process all potential JSON containers
            document.querySelectorAll('p, div, .analysis-summary, .llm-analysis div').forEach(element => {{
                if (processElementForJson(element)) {{
                    return; // Successfully processed as JSON
                }}
                
                // If not JSON, try markdown processing
                const text = element.textContent || element.innerText;
                if (text.includes('###') || text.includes('**') || text.includes('- ') || text.includes('#### ')) {{
                    const processedHtml = parseMarkdownContent(text);
                    element.innerHTML = processedHtml;
                    element.style.whiteSpace = 'normal';
                }}
            }});
        }});
    </script>
</head>
<body>
    <div class="header">
        <h1>Project Analysis Report</h1>
        <p><strong>Project:</strong> {}</p>
        <p><strong>Generated:</strong> {}</p>
        <p><strong>Analysis Duration:</strong> {}ms</p>
        <p><strong>LLM Model:</strong> {} ({})</p>
    </div>
    
    <div class="section">
        <h2>Executive Summary</h2>
        <div class="metric">
            <strong>Complexity Score:</strong> {:.2}
        </div>
        <div class="metric">
            <strong>Maintainability Score:</strong> {:.2}
        </div>
        <div class="metric">
            <strong>Total Files:</strong> {}
        </div>
        <div class="metric">
            <strong>Total Size:</strong> {:.2} MB
        </div>
        <p>{}</p>
    </div>

    <div class="section">
        <h2>Key Recommendations</h2>
        {}
    </div>

    <div class="section">
        <h2>LLM Analysis & Insights</h2>
        {}
    </div>

    <div class="section">
        <h2>File Analysis</h2>
        <h3>Language Distribution</h3>
        <table>
            <tr><th>Language</th><th>Files</th><th>Size (MB)</th><th>Percentage</th></tr>
            {}
        </table>
    </div>

</body>
</html>"#,
            report.metadata.project_name,
            report.metadata.project_name,
            report.metadata.generated_at,
            report.metadata.analysis_duration_ms,
            report.metadata.llm_model,
            report.metadata.llm_provider,
            report.executive_summary.complexity_score,
            report.executive_summary.maintainability_score,
            report.metadata.total_files,
            report.metadata.total_size as f64 / (1024.0 * 1024.0),
            report.executive_summary.overview,
            report.recommendations.iter().take(5).map(|r| {
                let priority_class = match r.priority {
                    Priority::High | Priority::Critical => "priority-high",
                    Priority::Medium => "priority-medium",
                    Priority::Low => "priority-low",
                };
                format!(r#"<div class="recommendation {}"><strong>{}</strong><p>{}</p></div>"#, 
                    priority_class, r.title, r.description)
            }).collect::<Vec<_>>().join("\n"),
            self.generate_llm_insights_html(&report.llm_insights),
            report.file_analysis.language_breakdown.iter().map(|l| {
                format!("<tr><td>{}</td><td>{}</td><td>{:.2}</td><td>{:.1}%</td></tr>",
                    l.language, l.file_count, l.total_size as f64 / (1024.0 * 1024.0), l.percentage)
            }).collect::<Vec<_>>().join("\n")
        );

        Ok(html)
    }

    fn generate_llm_insights_html(&self, llm_insights: &[AnalysisResponse]) -> String {
        if llm_insights.is_empty() {
            return "<p>No LLM analysis was performed for this project.</p>".to_string();
        }

        let mut html = String::new();
        
        for (index, analysis) in llm_insights.iter().enumerate() {
            // Determine analysis type from position (based on analyzer.rs order)
            let analysis_type = match index {
                0 => "Overview",
                1 => "Architecture", 
                2 => "Dependencies",
                3 => "Security",
                4 => "Refactoring",
                5 => "Documentation",
                _ => "Additional Analysis",
            };

            html.push_str(&format!(r#"<div class="llm-analysis">
                <div class="analysis-type">{} Analysis</div>"#, analysis_type));

            // Extract and display the main analysis summary
            let analysis_text = self.extract_analysis_text(&analysis.analysis);
            html.push_str(&format!(r#"<div class="analysis-summary">{}</div>"#, analysis_text));

            // Extract insights and display in table format
            let insights = if !analysis.insights.is_empty() {
                analysis.insights.clone()
            } else {
                self.extract_insights_from_text(&analysis.analysis)
            };

            if !insights.is_empty() {
                html.push_str(r#"<h4>Key Insights</h4>
                <table class="insights-table">
                    <thead>
                        <tr>
                            <th>Insight</th>
                            <th>Category</th>
                            <th>Description</th>
                            <th>Confidence</th>
                            <th>Evidence</th>
                        </tr>
                    </thead>
                    <tbody>"#);

                for insight in &insights {
                    let confidence_class = if insight.confidence >= 0.8 {
                        "confidence-high"
                    } else if insight.confidence >= 0.6 {
                        "confidence-medium"
                    } else {
                        "confidence-low"
                    };
                    
                    let evidence_text = if insight.evidence.is_empty() {
                        "No specific evidence".to_string()
                    } else {
                        insight.evidence.join("<br>• ")
                    };

                    html.push_str(&format!(r#"<tr>
                        <td><strong>{}</strong></td>
                        <td>{:?}</td>
                        <td>{}</td>
                        <td class="{}">{:.0}%</td>
                        <td>• {}</td>
                    </tr>"#, 
                    insight.title, insight.category, insight.description, 
                    confidence_class, insight.confidence * 100.0, evidence_text));
                }
                
                html.push_str("</tbody></table>");
            }

            // Extract recommendations and display in table format
            let recommendations = if !analysis.recommendations.is_empty() {
                analysis.recommendations.clone()
            } else {
                self.extract_recommendations_from_text(&analysis.analysis)
            };

            if !recommendations.is_empty() {
                html.push_str(r#"<h4>Recommendations</h4>
                <table class="recommendations-table">
                    <thead>
                        <tr>
                            <th>Title</th>
                            <th>Description</th>
                            <th>Priority</th>
                            <th>Effort</th>
                            <th>Impact</th>
                            <th>Action Items</th>
                        </tr>
                    </thead>
                    <tbody>"#);

                for recommendation in &recommendations {
                    let priority_class = match recommendation.priority {
                        Priority::High | Priority::Critical => "priority-high",
                        Priority::Medium => "priority-medium",
                        Priority::Low => "priority-low",
                    };
                    
                    let action_items_text = if recommendation.action_items.is_empty() {
                        "No specific actions".to_string()
                    } else {
                        recommendation.action_items.join("<br>• ")
                    };

                    html.push_str(&format!(r#"<tr class="{}">
                        <td><strong>{}</strong></td>
                        <td>{}</td>
                        <td>{:?}</td>
                        <td>{:?}</td>
                        <td>{:?}</td>
                        <td>• {}</td>
                    </tr>"#, 
                    priority_class, recommendation.title, recommendation.description,
                    recommendation.priority, recommendation.effort, recommendation.impact,
                    action_items_text));
                }
                
                html.push_str("</tbody></table>");
            }

            html.push_str("</div>");
        }

        html
    }

    fn extract_analysis_text(&self, content: &str) -> String {
        // First try to parse as JSON and extract the analysis field
        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(content) {
            if let Some(analysis_text) = json_value.get("analysis").and_then(|v| v.as_str()) {
                return analysis_text.to_string();
            }
        }

        // Check if content is wrapped in JSON code blocks
        if content.trim().starts_with("```json") && content.trim().ends_with("```") {
            let json_content = content.trim()
                .strip_prefix("```json").unwrap_or(content)
                .strip_suffix("```").unwrap_or(content)
                .trim();
            
            if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(json_content) {
                if let Some(analysis_text) = json_value.get("analysis").and_then(|v| v.as_str()) {
                    return analysis_text.to_string();
                }
            }
        }

        // If not JSON, return the content as is (cleaning up markdown if needed)
        content.to_string()
    }


    fn extract_insights_from_text(&self, text: &str) -> Vec<crate::llm::Insight> {
        // Try to parse JSON and extract insights
        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(text) {
            if let Some(insights_array) = json_value.get("insights").and_then(|v| v.as_array()) {
                let mut insights = Vec::new();
                for insight_json in insights_array {
                    if let Ok(insight) = serde_json::from_value::<crate::llm::Insight>(insight_json.clone()) {
                        insights.push(insight);
                    }
                }
                return insights;
            }
        }

        // Try to parse JSON wrapped in code blocks
        if text.trim().starts_with("```json") && text.trim().ends_with("```") {
            let json_content = text.trim()
                .strip_prefix("```json").unwrap_or(text)
                .strip_suffix("```").unwrap_or(text)
                .trim();
            
            if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(json_content) {
                if let Some(insights_array) = json_value.get("insights").and_then(|v| v.as_array()) {
                    let mut insights = Vec::new();
                    for insight_json in insights_array {
                        if let Ok(insight) = serde_json::from_value::<crate::llm::Insight>(insight_json.clone()) {
                            insights.push(insight);
                        }
                    }
                    return insights;
                }
            }
        }

        Vec::new()
    }

    fn extract_recommendations_from_text(&self, text: &str) -> Vec<crate::llm::Recommendation> {
        // Try to parse JSON and extract recommendations
        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(text) {
            if let Some(recommendations_array) = json_value.get("recommendations").and_then(|v| v.as_array()) {
                let mut recommendations = Vec::new();
                for rec_json in recommendations_array {
                    if let Ok(recommendation) = serde_json::from_value::<crate::llm::Recommendation>(rec_json.clone()) {
                        recommendations.push(recommendation);
                    }
                }
                return recommendations;
            }
        }

        // Try to parse JSON wrapped in code blocks
        if text.trim().starts_with("```json") && text.trim().ends_with("```") {
            let json_content = text.trim()
                .strip_prefix("```json").unwrap_or(text)
                .strip_suffix("```").unwrap_or(text)
                .trim();
            
            if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(json_content) {
                if let Some(recommendations_array) = json_value.get("recommendations").and_then(|v| v.as_array()) {
                    let mut recommendations = Vec::new();
                    for rec_json in recommendations_array {
                        if let Ok(recommendation) = serde_json::from_value::<crate::llm::Recommendation>(rec_json.clone()) {
                            recommendations.push(recommendation);
                        }
                    }
                    return recommendations;
                }
            }
        }

        Vec::new()
    }

    fn generate_markdown_summary(&self, report: &Report) -> Result<String> {
        let mut md = format!(
            "# Project Analysis Summary\n\n**Project:** {}\n**Generated:** {}\n**Analysis Duration:** {}ms\n\n",
            report.metadata.project_name,
            report.metadata.generated_at,
            report.metadata.analysis_duration_ms
        );

        md.push_str("## Executive Summary\n\n");
        md.push_str(&format!("- **Complexity Score:** {:.2}/10\n", report.executive_summary.complexity_score));
        md.push_str(&format!("- **Maintainability Score:** {:.2}/10\n", report.executive_summary.maintainability_score));
        md.push_str(&format!("- **Total Files:** {}\n", report.metadata.total_files));
        md.push_str(&format!("- **Total Size:** {:.2} MB\n\n", report.metadata.total_size as f64 / (1024.0 * 1024.0)));

        md.push_str("## Top Recommendations\n\n");
        for (i, rec) in report.recommendations.iter().take(5).enumerate() {
            md.push_str(&format!("{}. **{}** (Priority: {:?})\n   {}\n\n", 
                i + 1, rec.title, rec.priority, rec.description));
        }

        md.push_str("## Language Distribution\n\n");
        for lang in &report.file_analysis.language_breakdown {
            md.push_str(&format!("- **{}:** {} files ({:.1}%), {:.2} MB\n", 
                lang.language, lang.file_count, lang.percentage, lang.total_size as f64 / (1024.0 * 1024.0)));
        }

        Ok(md)
    }
}