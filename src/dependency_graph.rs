use crate::simple_parser::{ParsedFile, Function, Class};
use petgraph::{Graph, Directed, graph::NodeIndex};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

pub type DependencyGraph = Graph<Node, Edge, Directed>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub node_type: NodeType,
    pub file_path: PathBuf,
    pub line_number: usize,
    pub metadata: NodeMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeType {
    File,
    Module,
    Function,
    Class,
    Variable,
    Import,
    Export,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMetadata {
    pub name: String,
    pub language: Option<String>,
    pub size: Option<u64>,
    pub complexity: Option<usize>,
    pub parameters: Vec<String>,
    pub return_type: Option<String>,
    pub is_async: bool,
    pub is_exported: bool,
    pub docstring: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub edge_type: EdgeType,
    pub weight: f64,
    pub metadata: EdgeMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EdgeType {
    Imports,
    Calls,
    Extends,
    Implements,
    Contains,
    References,
    DependsOn,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeMetadata {
    pub call_count: usize,
    pub is_direct: bool,
    pub line_numbers: Vec<usize>,
}

pub struct GraphBuilder {
    graph: DependencyGraph,
    node_map: HashMap<String, NodeIndex>,
    file_nodes: HashMap<PathBuf, NodeIndex>,
}

impl GraphBuilder {
    pub fn new() -> Self {
        Self {
            graph: Graph::new(),
            node_map: HashMap::new(),
            file_nodes: HashMap::new(),
        }
    }

    pub fn build_graph(&mut self, parsed_files: &[ParsedFile]) -> &DependencyGraph {
        for parsed_file in parsed_files {
            self.add_file_node(parsed_file);
            self.add_imports(parsed_file);
            self.add_functions(parsed_file);
            self.add_classes(parsed_file);
        }

        self.add_call_relationships(parsed_files);
        &self.graph
    }

    fn add_file_node(&mut self, parsed_file: &ParsedFile) {
        let node_id = format!("file:{}", parsed_file.file_info.path.display());
        
        let node = Node {
            id: node_id.clone(),
            node_type: NodeType::File,
            file_path: parsed_file.file_info.path.clone(),
            line_number: 1,
            metadata: NodeMetadata {
                name: parsed_file.file_info.path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string(),
                language: parsed_file.file_info.language.clone(),
                size: Some(parsed_file.file_info.size),
                complexity: Some(self.calculate_file_complexity(parsed_file)),
                parameters: Vec::new(),
                return_type: None,
                is_async: false,
                is_exported: false,
                docstring: None,
            },
        };

        let node_index = self.graph.add_node(node);
        self.node_map.insert(node_id, node_index);
        self.file_nodes.insert(parsed_file.file_info.path.clone(), node_index);
    }

    fn add_imports(&mut self, parsed_file: &ParsedFile) {
        let file_node = self.file_nodes[&parsed_file.file_info.path];

        for import in &parsed_file.imports {
            let import_id = format!("import:{}:{}", parsed_file.file_info.path.display(), import.module);
            
            let node = Node {
                id: import_id.clone(),
                node_type: NodeType::Import,
                file_path: parsed_file.file_info.path.clone(),
                line_number: import.line_number,
                metadata: NodeMetadata {
                    name: import.module.clone(),
                    language: parsed_file.file_info.language.clone(),
                    size: None,
                    complexity: None,
                    parameters: import.items.clone(),
                    return_type: None,
                    is_async: false,
                    is_exported: false,
                    docstring: None,
                },
            };

            let import_node = self.graph.add_node(node);
            self.node_map.insert(import_id, import_node);

            let edge = Edge {
                edge_type: EdgeType::Contains,
                weight: 1.0,
                metadata: EdgeMetadata {
                    call_count: 1,
                    is_direct: true,
                    line_numbers: vec![import.line_number],
                },
            };

            self.graph.add_edge(file_node, import_node, edge);
        }
    }

    fn add_functions(&mut self, parsed_file: &ParsedFile) {
        let file_node = self.file_nodes[&parsed_file.file_info.path];

        for function in &parsed_file.functions {
            let function_id = format!("function:{}:{}", parsed_file.file_info.path.display(), function.name);
            
            let node = Node {
                id: function_id.clone(),
                node_type: NodeType::Function,
                file_path: parsed_file.file_info.path.clone(),
                line_number: function.line_number,
                metadata: NodeMetadata {
                    name: function.name.clone(),
                    language: parsed_file.file_info.language.clone(),
                    size: None,
                    complexity: Some(self.calculate_function_complexity(function)),
                    parameters: function.parameters.clone(),
                    return_type: function.return_type.clone(),
                    is_async: function.is_async,
                    is_exported: self.is_function_exported(parsed_file, function),
                    docstring: None,
                },
            };

            let function_node = self.graph.add_node(node);
            self.node_map.insert(function_id, function_node);

            let edge = Edge {
                edge_type: EdgeType::Contains,
                weight: 1.0,
                metadata: EdgeMetadata {
                    call_count: 1,
                    is_direct: true,
                    line_numbers: vec![function.line_number],
                },
            };

            self.graph.add_edge(file_node, function_node, edge);
        }
    }

    fn add_classes(&mut self, parsed_file: &ParsedFile) {
        let file_node = self.file_nodes[&parsed_file.file_info.path];

        for class in &parsed_file.classes {
            let class_id = format!("class:{}:{}", parsed_file.file_info.path.display(), class.name);
            
            let node = Node {
                id: class_id.clone(),
                node_type: NodeType::Class,
                file_path: parsed_file.file_info.path.clone(),
                line_number: class.line_number,
                metadata: NodeMetadata {
                    name: class.name.clone(),
                    language: parsed_file.file_info.language.clone(),
                    size: None,
                    complexity: Some(self.calculate_class_complexity(class)),
                    parameters: Vec::new(),
                    return_type: None,
                    is_async: false,
                    is_exported: self.is_class_exported(parsed_file, class),
                    docstring: None,
                },
            };

            let class_node = self.graph.add_node(node);
            self.node_map.insert(class_id, class_node);

            let edge = Edge {
                edge_type: EdgeType::Contains,
                weight: 1.0,
                metadata: EdgeMetadata {
                    call_count: 1,
                    is_direct: true,
                    line_numbers: vec![class.line_number],
                },
            };

            self.graph.add_edge(file_node, class_node, edge);

            for method in &class.methods {
                let method_id = format!("method:{}:{}:{}", parsed_file.file_info.path.display(), class.name, method.name);
                
                let method_node_data = Node {
                    id: method_id.clone(),
                    node_type: NodeType::Function,
                    file_path: parsed_file.file_info.path.clone(),
                    line_number: method.line_number,
                    metadata: NodeMetadata {
                        name: format!("{}.{}", class.name, method.name),
                        language: parsed_file.file_info.language.clone(),
                        size: None,
                        complexity: Some(self.calculate_function_complexity(method)),
                        parameters: method.parameters.clone(),
                        return_type: method.return_type.clone(),
                        is_async: method.is_async,
                        is_exported: false,
                        docstring: None,
                    },
                };

                let method_node = self.graph.add_node(method_node_data);
                self.node_map.insert(method_id, method_node);

                let method_edge = Edge {
                    edge_type: EdgeType::Contains,
                    weight: 1.0,
                    metadata: EdgeMetadata {
                        call_count: 1,
                        is_direct: true,
                        line_numbers: vec![method.line_number],
                    },
                };

                self.graph.add_edge(class_node, method_node, method_edge);
            }
        }
    }

    fn add_call_relationships(&mut self, parsed_files: &[ParsedFile]) {
        for parsed_file in parsed_files {
            for import in &parsed_file.imports {
                if let Some(target_file) = self.find_imported_file(parsed_files, &import.module) {
                    if let Some(&import_node) = self.node_map.get(&format!("import:{}:{}", parsed_file.file_info.path.display(), import.module)) {
                        if let Some(&target_node) = self.file_nodes.get(&target_file.file_info.path) {
                            let edge = Edge {
                                edge_type: EdgeType::DependsOn,
                                weight: 1.0,
                                metadata: EdgeMetadata {
                                    call_count: 1,
                                    is_direct: true,
                                    line_numbers: vec![import.line_number],
                                },
                            };

                            self.graph.add_edge(import_node, target_node, edge);
                        }
                    }
                }
            }
        }
    }

    fn find_imported_file<'a>(&self, parsed_files: &'a [ParsedFile], module_name: &str) -> Option<&'a ParsedFile> {
        parsed_files.iter().find(|f| {
            f.file_info.path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s == module_name)
                .unwrap_or(false)
        })
    }

    fn calculate_file_complexity(&self, parsed_file: &ParsedFile) -> usize {
        parsed_file.functions.len() + parsed_file.classes.len() + parsed_file.imports.len()
    }

    fn calculate_function_complexity(&self, function: &Function) -> usize {
        function.parameters.len() + if function.is_async { 2 } else { 1 }
    }

    fn calculate_class_complexity(&self, class: &Class) -> usize {
        class.methods.len() + class.implements.len() + if class.extends.is_some() { 1 } else { 0 }
    }

    fn is_function_exported(&self, parsed_file: &ParsedFile, function: &Function) -> bool {
        parsed_file.exports.iter().any(|e| e.name == function.name)
    }

    fn is_class_exported(&self, parsed_file: &ParsedFile, class: &Class) -> bool {
        parsed_file.exports.iter().any(|e| e.name == class.name)
    }

    pub fn get_graph(&self) -> &DependencyGraph {
        &self.graph
    }

    pub fn get_node_map(&self) -> &HashMap<String, NodeIndex> {
        &self.node_map
    }

    pub fn analyze_dependencies(&self) -> DependencyAnalysis {
        let total_nodes = self.graph.node_count();
        let total_edges = self.graph.edge_count();
        
        let mut node_types = HashMap::new();
        let mut edge_types = HashMap::new();
        let strongly_connected_components = 0;
        
        for node_weight in self.graph.node_weights() {
            *node_types.entry(format!("{:?}", node_weight.node_type)).or_insert(0) += 1;
        }
        
        for edge_weight in self.graph.edge_weights() {
            *edge_types.entry(format!("{:?}", edge_weight.edge_type)).or_insert(0) += 1;
        }

        DependencyAnalysis {
            total_nodes,
            total_edges,
            node_types,
            edge_types,
            strongly_connected_components,
            avg_degree: if total_nodes > 0 { total_edges as f64 / total_nodes as f64 } else { 0.0 },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyAnalysis {
    pub total_nodes: usize,
    pub total_edges: usize,
    pub node_types: HashMap<String, usize>,
    pub edge_types: HashMap<String, usize>,
    pub strongly_connected_components: usize,
    pub avg_degree: f64,
}

impl DependencyAnalysis {
    pub fn print_summary(&self) {
        println!("Dependency Graph Analysis:");
        println!("  Total nodes: {}", self.total_nodes);
        println!("  Total edges: {}", self.total_edges);
        println!("  Average degree: {:.2}", self.avg_degree);
        
        println!("  Node types:");
        for (node_type, count) in &self.node_types {
            println!("    {}: {}", node_type, count);
        }
        
        println!("  Edge types:");
        for (edge_type, count) in &self.edge_types {
            println!("    {}: {}", edge_type, count);
        }
    }
}