use crate::config::Config;
use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs;
use regex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: PathBuf,
    pub size: u64,
    pub extension: Option<String>,
    pub language: Option<String>,
}

pub struct FileDiscovery {
    config: Config,
}

impl FileDiscovery {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub fn discover_files(&self) -> crate::Result<Vec<FileInfo>> {
        let mut files = Vec::new();
        
        let mut walker_builder = WalkBuilder::new(&self.config.target_directory);
        walker_builder
            .standard_filters(true)  // This enables .gitignore support
            .hidden(false)           // Show hidden files except those in .gitignore
            .git_ignore(true)        // Explicitly enable .gitignore parsing
            .git_global(true)        // Respect global git ignore
            .git_exclude(true);      // Respect .git/info/exclude
            
        // The ignore patterns will be handled in the file processing logic
        
        let walker = walker_builder.build();

        for result in walker {
            let entry = result?;
            let path = entry.path();
            
            if !path.is_file() {
                continue;
            }

            // Check if file matches any ignore patterns
            if self.should_ignore_file(path) {
                continue;
            }

            if let Some(file_info) = self.process_file(path)? {
                files.push(file_info);
            }
        }

        Ok(files)
    }

    fn should_ignore_file(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        
        for pattern in &self.config.ignore_patterns {
            // Handle simple glob patterns (*.ext)
            if pattern.starts_with("*.") {
                if let Some(filename) = path.file_name() {
                    let filename_str = filename.to_string_lossy();
                    let ext = &pattern[2..]; // Remove "*."
                    if filename_str.ends_with(&format!(".{}", ext)) {
                        return true;
                    }
                }
            } else if pattern.contains('*') {
                // Handle other wildcard patterns by converting to simple regex
                let regex_pattern = pattern.replace('*', ".*");
                if let Ok(re) = regex::Regex::new(&regex_pattern) {
                    if re.is_match(&path_str) {
                        return true;
                    }
                    if let Some(filename) = path.file_name() {
                        if re.is_match(&filename.to_string_lossy()) {
                            return true;
                        }
                    }
                }
            } else {
                // Handle exact matches and directory names
                if path_str.contains(pattern) {
                    return true;
                }
                // Check if any component of the path matches
                for component in path.components() {
                    if component.as_os_str().to_string_lossy() == *pattern {
                        return true;
                    }
                }
            }
        }
        
        false
    }

    fn process_file(&self, path: &Path) -> crate::Result<Option<FileInfo>> {
        let metadata = fs::metadata(path)?;
        let size = metadata.len();

        if size > self.config.max_file_size as u64 {
            return Ok(None);
        }

        let extension = path.extension()
            .and_then(|ext| ext.to_str())
            .map(|s| s.to_lowercase());

        if let Some(ref ext) = extension {
            if !self.config.file_extensions.contains(ext) {
                return Ok(None);
            }
        }

        let language = self.detect_language(path, &extension);

        Ok(Some(FileInfo {
            path: path.to_path_buf(),
            size,
            extension,
            language,
        }))
    }

    fn detect_language(&self, path: &Path, extension: &Option<String>) -> Option<String> {
        // Handle files without extensions by filename
        if extension.is_none() {
            if let Some(filename) = path.file_name() {
                let filename_lower = filename.to_string_lossy().to_lowercase();
                match filename_lower.as_str() {
                    "readme" | "license" | "changelog" | "contributing" | "authors" | 
                    "install" | "usage" | "todo" | "news" | "history" | "acknowledgments" |
                    "makefile" | "dockerfile" => return Some("text".to_string()),
                    _ => {}
                }
            }
        }
        
        match extension.as_deref() {
            Some("rs") => Some("rust".to_string()),
            Some("js") => Some("javascript".to_string()),
            Some("ts") => Some("typescript".to_string()),
            Some("tsx") => Some("typescript".to_string()),
            Some("jsx") => Some("javascript".to_string()),
            Some("py") => Some("python".to_string()),
            Some("java") => Some("java".to_string()),
            Some("go") => Some("go".to_string()),
            Some("cpp") | Some("cc") | Some("cxx") => Some("cpp".to_string()),
            Some("c") => Some("c".to_string()),
            Some("h") | Some("hpp") => Some("c".to_string()),
            Some("php") => Some("php".to_string()),
            Some("rb") => Some("ruby".to_string()),
            Some("cs") => Some("csharp".to_string()),
            Some("swift") => Some("swift".to_string()),
            Some("kt") => Some("kotlin".to_string()),
            Some("scala") => Some("scala".to_string()),
            Some("clj") | Some("cljs") => Some("clojure".to_string()),
            Some("hs") => Some("haskell".to_string()),
            Some("ml") | Some("mli") => Some("ocaml".to_string()),
            Some("elm") => Some("elm".to_string()),
            Some("ex") | Some("exs") => Some("elixir".to_string()),
            Some("erl") | Some("hrl") => Some("erlang".to_string()),
            Some("dart") => Some("dart".to_string()),
            Some("lua") => Some("lua".to_string()),
            Some("r") => Some("r".to_string()),
            Some("m") => Some("objective-c".to_string()),
            Some("mm") => Some("objective-cpp".to_string()),
            Some("pl") | Some("pm") => Some("perl".to_string()),
            Some("sh") | Some("bash") => Some("bash".to_string()),
            Some("ps1") => Some("powershell".to_string()),
            Some("sql") => Some("sql".to_string()),
            Some("html") | Some("htm") => Some("html".to_string()),
            Some("css") => Some("css".to_string()),
            Some("scss") | Some("sass") => Some("scss".to_string()),
            Some("xml") => Some("xml".to_string()),
            Some("json") => Some("json".to_string()),
            Some("yaml") | Some("yml") => Some("yaml".to_string()),
            Some("toml") => Some("toml".to_string()),
            Some("md") => Some("markdown".to_string()),
            Some("txt") => Some("text".to_string()),
            Some("tex") => Some("latex".to_string()),
            Some("dockerfile") => Some("dockerfile".to_string()),
            Some("makefile") => Some("makefile".to_string()),
            Some("cmake") => Some("cmake".to_string()),
            _ => None,
        }
    }

    pub fn filter_by_language<'a>(&self, files: &'a [FileInfo], language: &str) -> Vec<&'a FileInfo> {
        files.iter()
            .filter(|f| f.language.as_deref() == Some(language))
            .collect()
    }

    pub fn get_stats(&self, files: &[FileInfo]) -> FileStats {
        let mut stats = FileStats::default();
        
        for file in files {
            stats.total_files += 1;
            stats.total_size += file.size;
            
            if let Some(ref lang) = file.language {
                *stats.languages.entry(lang.clone()).or_insert(0) += 1;
            }
        }
        
        stats
    }
}

#[derive(Debug, Default)]
pub struct FileStats {
    pub total_files: usize,
    pub total_size: u64,
    pub languages: std::collections::HashMap<String, usize>,
}

impl FileStats {
    pub fn print_summary(&self) {
        println!("File Discovery Summary:");
        println!("  Total files: {}", self.total_files);
        println!("  Total size: {:.2} MB", self.total_size as f64 / (1024.0 * 1024.0));
        println!("  Languages:");
        
        let mut langs: Vec<_> = self.languages.iter().collect();
        langs.sort_by(|a, b| b.1.cmp(a.1));
        
        for (lang, count) in langs {
            println!("    {}: {} files", lang, count);
        }
    }
}