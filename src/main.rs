use project_examer::{Config, Analyzer, Reporter};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::time::Instant;

#[derive(Parser)]
#[command(name = "project-examer")]
#[command(about = "A fast system analysis tool for scanning and analyzing codebases")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Analyze a project directory
    Analyze {
        /// Target directory to analyze
        #[arg(short, long, default_value = ".")]
        path: PathBuf,
        
        /// Configuration file path
        #[arg(short, long)]
        config: Option<PathBuf>,
        
        /// Output directory for reports
        #[arg(short, long, default_value = "./analysis-output")]
        output: PathBuf,
        
        /// Skip LLM analysis (faster, local-only analysis)
        #[arg(long)]
        skip_llm: bool,
        
        /// Generate only specific report format
        #[arg(long, value_enum)]
        format: Option<ReportFormat>,
    },
    /// Generate a default configuration file
    Config {
        /// Output path for the config file (defaults to ~/.project-examer.toml)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

#[derive(clap::ValueEnum, Clone)]
enum ReportFormat {
    Json,
    Html,
    Markdown,
    All,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Analyze { path, config, output, skip_llm, format } => {
            analyze_project(path, config, output, skip_llm, format).await?;
        }
        Commands::Config { output } => {
            generate_config(output)?;
        }
    }

    Ok(())
}

async fn analyze_project(
    target_path: PathBuf,
    config_path: Option<PathBuf>,
    output_path: PathBuf,
    skip_llm: bool,
    _format: Option<ReportFormat>,
) -> anyhow::Result<()> {
    println!("🚀 Starting Project Examer Analysis");
    println!("====================================");
    
    let start_time = Instant::now();
    
    // Load configuration
    let mut config = if let Some(config_path) = config_path {
        Config::from_file(&config_path)?
    } else {
        Config::load()?
    };
    
    // Override target directory
    config.target_directory = target_path.clone();
    
    println!("🎯 Target directory: {}", target_path.display());
    println!("📤 Output directory: {}", output_path.display());
    
    if skip_llm {
        println!("⚡ Skipping LLM analysis (local-only mode)");
        config.llm.provider = project_examer::config::LLMProvider::OpenAI; // Will be unused
    }

    // Initialize analyzer
    let mut analyzer = Analyzer::new(config)?;
    
    // Run analysis
    let analysis = analyzer.analyze_project(skip_llm).await?;
    
    let duration = start_time.elapsed();
    
    // Print summary
    analysis.print_summary();
    
    // Generate reports
    println!("\n📊 Generating reports...");
    let reporter = Reporter::new();
    let report = reporter.generate_report(&analysis, duration.as_millis());
    let exported_files = reporter.export_report(&report, &output_path)?;
    
    println!("\n✅ Analysis completed in {:.2}s", duration.as_secs_f64());
    println!("📁 Reports exported to:");
    for file in exported_files {
        println!("   - {}", file.display());
    }
    
    Ok(())
}

fn generate_config(output_path: Option<PathBuf>) -> anyhow::Result<()> {
    let config_path = output_path.unwrap_or_else(|| {
        Config::default_config_path().unwrap_or_else(|_| PathBuf::from("project-examer.toml"))
    });
    
    println!("📝 Generating configuration file: {}", config_path.display());
    
    // Write the documented config instead of default
    let documented_config = Config::create_documented_config();
    std::fs::write(&config_path, documented_config)?;
    
    println!("✅ Configuration file created successfully!");
    println!("💡 Edit the file to customize your analysis settings.");
    println!();
    println!("🔧 Key configuration areas:");
    println!("  • LLM provider settings (OpenAI, Anthropic, Ollama)");
    println!("  • File patterns and extensions to analyze");
    println!("  • Analysis options and security scanning");
    println!("  • API keys (or use environment variables)");
    
    Ok(())
}
