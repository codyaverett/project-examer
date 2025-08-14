use crate::file_discovery::FileInfo;
use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedFile {
    pub file_info: FileInfo,
    pub imports: Vec<Import>,
    pub exports: Vec<Export>,
    pub functions: Vec<Function>,
    pub classes: Vec<Class>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Import {
    pub module: String,
    pub items: Vec<String>,
    pub is_default: bool,
    pub line_number: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Export {
    pub name: String,
    pub is_default: bool,
    pub line_number: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Function {
    pub name: String,
    pub parameters: Vec<String>,
    pub return_type: Option<String>,
    pub line_number: usize,
    pub is_async: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Class {
    pub name: String,
    pub extends: Option<String>,
    pub implements: Vec<String>,
    pub methods: Vec<Function>,
    pub line_number: usize,
}

pub struct SimpleParser {
    language_patterns: HashMap<String, LanguagePatterns>,
}

struct LanguagePatterns {
    import_patterns: Vec<Regex>,
    export_patterns: Vec<Regex>,
    function_patterns: Vec<Regex>,
    class_patterns: Vec<Regex>,
}

impl SimpleParser {
    pub fn new() -> Result<Self> {
        let mut language_patterns = HashMap::new();
        
        // JavaScript/TypeScript patterns
        language_patterns.insert("javascript".to_string(), LanguagePatterns {
            import_patterns: vec![
                Regex::new(r#"import\s+.*?\s+from\s+['"]([^'"]+)['"]"#)?,
                Regex::new(r#"import\s+['"]([^'"]+)['"]"#)?,
                Regex::new(r#"const\s+.*?\s*=\s*require\s*\(\s*['"]([^'"]+)['"]"#)?,
            ],
            export_patterns: vec![
                Regex::new(r"export\s+(function|class|const|let|var)\s+(\w+)")?,
                Regex::new(r"export\s+default\s+(\w+)")?,
                Regex::new(r"export\s*\{\s*([^}]+)\s*\}")?,
            ],
            function_patterns: vec![
                Regex::new(r"function\s+(\w+)\s*\(([^)]*)\)")?,
                Regex::new(r"(\w+)\s*:\s*function\s*\(([^)]*)\)")?,
                Regex::new(r"(\w+)\s*=>\s*")?,
                Regex::new(r"(async\s+)?function\s+(\w+)")?,
            ],
            class_patterns: vec![
                Regex::new(r"class\s+(\w+)(?:\s+extends\s+(\w+))?")?,
            ],
        });
        
        // TypeScript (same as JavaScript with some additions)
        language_patterns.insert("typescript".to_string(), language_patterns["javascript"].clone());
        
        // Python patterns
        language_patterns.insert("python".to_string(), LanguagePatterns {
            import_patterns: vec![
                Regex::new(r"from\s+([^\s]+)\s+import")?,
                Regex::new(r"import\s+([^\s,]+)")?,
            ],
            export_patterns: vec![
                Regex::new(r"__all__\s*=\s*\[([^\]]+)\]")?,
            ],
            function_patterns: vec![
                Regex::new(r"def\s+(\w+)\s*\(([^)]*)\)")?,
                Regex::new(r"async\s+def\s+(\w+)\s*\(([^)]*)\)")?,
            ],
            class_patterns: vec![
                Regex::new(r"class\s+(\w+)(?:\(([^)]+)\))?")?,
            ],
        });
        
        // Rust patterns
        language_patterns.insert("rust".to_string(), LanguagePatterns {
            import_patterns: vec![
                Regex::new(r"use\s+([^;]+);")?,
                Regex::new(r"extern\s+crate\s+(\w+)")?,
            ],
            export_patterns: vec![
                Regex::new(r"pub\s+(fn|struct|enum|trait|mod)\s+(\w+)")?,
            ],
            function_patterns: vec![
                Regex::new(r"fn\s+(\w+)\s*\(([^)]*)\)")?,
                Regex::new(r"pub\s+fn\s+(\w+)\s*\(([^)]*)\)")?,
                Regex::new(r"async\s+fn\s+(\w+)")?,
            ],
            class_patterns: vec![
                Regex::new(r"struct\s+(\w+)")?,
                Regex::new(r"enum\s+(\w+)")?,
                Regex::new(r"trait\s+(\w+)")?,
            ],
        });
        
        Ok(Self { language_patterns })
    }

    pub fn parse_file(&self, file_info: &FileInfo) -> Result<ParsedFile> {
        let content = std::fs::read_to_string(&file_info.path)?;
        
        let default_language = "unknown".to_string();
        let language = file_info.language.as_ref()
            .unwrap_or(&default_language);

        let patterns = self.language_patterns.get(language);
        
        let mut parsed_file = ParsedFile {
            file_info: file_info.clone(),
            imports: Vec::new(),
            exports: Vec::new(),
            functions: Vec::new(),
            classes: Vec::new(),
        };

        if let Some(patterns) = patterns {
            self.extract_imports(&content, patterns, &mut parsed_file)?;
            self.extract_exports(&content, patterns, &mut parsed_file)?;
            self.extract_functions(&content, patterns, &mut parsed_file)?;
            self.extract_classes(&content, patterns, &mut parsed_file)?;
        } else {
            // Fallback: basic pattern matching for unknown languages
            self.extract_basic_patterns(&content, &mut parsed_file)?;
        }

        Ok(parsed_file)
    }

    fn extract_imports(&self, content: &str, patterns: &LanguagePatterns, parsed_file: &mut ParsedFile) -> Result<()> {
        for (line_num, line) in content.lines().enumerate() {
            for pattern in &patterns.import_patterns {
                if let Some(captures) = pattern.captures(line) {
                    if let Some(module) = captures.get(1) {
                        parsed_file.imports.push(Import {
                            module: module.as_str().to_string(),
                            items: Vec::new(),
                            is_default: false,
                            line_number: line_num + 1,
                        });
                    }
                }
            }
        }
        Ok(())
    }

    fn extract_exports(&self, content: &str, patterns: &LanguagePatterns, parsed_file: &mut ParsedFile) -> Result<()> {
        for (line_num, line) in content.lines().enumerate() {
            for pattern in &patterns.export_patterns {
                if let Some(captures) = pattern.captures(line) {
                    if let Some(name) = captures.get(captures.len() - 1) {
                        parsed_file.exports.push(Export {
                            name: name.as_str().to_string(),
                            is_default: line.contains("default"),
                            line_number: line_num + 1,
                        });
                    }
                }
            }
        }
        Ok(())
    }

    fn extract_functions(&self, content: &str, patterns: &LanguagePatterns, parsed_file: &mut ParsedFile) -> Result<()> {
        for (line_num, line) in content.lines().enumerate() {
            for pattern in &patterns.function_patterns {
                if let Some(captures) = pattern.captures(line) {
                    let is_async = line.contains("async");
                    let name = if captures.len() > 2 {
                        captures.get(2).map(|m| m.as_str()).unwrap_or("unknown")
                    } else {
                        captures.get(1).map(|m| m.as_str()).unwrap_or("unknown")
                    };
                    
                    let params = if captures.len() > 2 {
                        captures.get(captures.len() - 1)
                    } else {
                        captures.get(2)
                    };
                    
                    let parameters = if let Some(params) = params {
                        self.parse_parameters(params.as_str())
                    } else {
                        Vec::new()
                    };

                    parsed_file.functions.push(Function {
                        name: name.to_string(),
                        parameters,
                        return_type: None,
                        line_number: line_num + 1,
                        is_async,
                    });
                }
            }
        }
        Ok(())
    }

    fn extract_classes(&self, content: &str, patterns: &LanguagePatterns, parsed_file: &mut ParsedFile) -> Result<()> {
        for (line_num, line) in content.lines().enumerate() {
            for pattern in &patterns.class_patterns {
                if let Some(captures) = pattern.captures(line) {
                    if let Some(name) = captures.get(1) {
                        let extends = captures.get(2).map(|m| m.as_str().to_string());
                        
                        parsed_file.classes.push(Class {
                            name: name.as_str().to_string(),
                            extends,
                            implements: Vec::new(),
                            methods: Vec::new(),
                            line_number: line_num + 1,
                        });
                    }
                }
            }
        }
        Ok(())
    }

    fn extract_basic_patterns(&self, content: &str, parsed_file: &mut ParsedFile) -> Result<()> {
        // Basic patterns that work across languages
        let import_patterns = [
            r#"import.*['"]([^'"]+)['"]"#,
            r#"#include\s*[<"]([^>"]+)[>"]"#,
            r#"require\s*\(['"]([^'"]+)['"]\)"#,
        ];
        
        let function_patterns = [
            r"(function|def|fn)\s+(\w+)",
            r"(\w+)\s*\(",
        ];

        for (line_num, line) in content.lines().enumerate() {
            // Try to find imports
            for pattern_str in &import_patterns {
                if let Ok(pattern) = Regex::new(pattern_str) {
                    if let Some(captures) = pattern.captures(line) {
                        if let Some(module) = captures.get(1) {
                            parsed_file.imports.push(Import {
                                module: module.as_str().to_string(),
                                items: Vec::new(),
                                is_default: false,
                                line_number: line_num + 1,
                            });
                        }
                    }
                }
            }
            
            // Try to find functions
            for pattern_str in &function_patterns {
                if let Ok(pattern) = Regex::new(pattern_str) {
                    if let Some(captures) = pattern.captures(line) {
                        if let Some(name) = captures.get(2).or(captures.get(1)) {
                            parsed_file.functions.push(Function {
                                name: name.as_str().to_string(),
                                parameters: Vec::new(),
                                return_type: None,
                                line_number: line_num + 1,
                                is_async: line.contains("async"),
                            });
                        }
                    }
                }
            }
        }
        
        Ok(())
    }

    fn parse_parameters(&self, params_str: &str) -> Vec<String> {
        params_str
            .split(',')
            .map(|p| {
                // Extract parameter name (before : or = if present)
                p.trim()
                    .split(':')
                    .next()
                    .unwrap_or(p.trim())
                    .split('=')
                    .next()
                    .unwrap_or(p.trim())
                    .trim()
                    .to_string()
            })
            .filter(|p| !p.is_empty())
            .collect()
    }

    pub fn get_dependencies(&self, parsed_file: &ParsedFile) -> Vec<String> {
        parsed_file.imports.iter()
            .map(|import| import.module.clone())
            .collect()
    }
}

impl Clone for LanguagePatterns {
    fn clone(&self) -> Self {
        Self {
            import_patterns: self.import_patterns.iter().map(|r| Regex::new(r.as_str()).unwrap()).collect(),
            export_patterns: self.export_patterns.iter().map(|r| Regex::new(r.as_str()).unwrap()).collect(),
            function_patterns: self.function_patterns.iter().map(|r| Regex::new(r.as_str()).unwrap()).collect(),
            class_patterns: self.class_patterns.iter().map(|r| Regex::new(r.as_str()).unwrap()).collect(),
        }
    }
}