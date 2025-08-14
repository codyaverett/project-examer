use project_examer::Config;

fn main() -> anyhow::Result<()> {
    println!("Project Examer Configuration Example");
    println!("====================================");
    
    // Show default config path
    match Config::default_config_path() {
        Ok(path) => println!("📍 Default config location: {}", path.display()),
        Err(e) => println!("❌ Error getting config path: {}", e),
    }
    
    // Load config (will use defaults if no file exists)
    println!("\n🔧 Loading configuration...");
    let config = Config::load()?;
    
    println!("✅ Configuration loaded successfully!");
    println!("📁 Target directory: {}", config.target_directory.display());
    println!("🔍 File extensions: {:?}", config.file_extensions);
    println!("🚫 Ignore patterns: {:?}", config.ignore_patterns);
    println!("🤖 LLM Provider: {:?}", config.llm.provider);
    println!("🧠 Model: {}", config.llm.model);
    
    if config.llm.api_key.is_some() {
        println!("🔑 API key: [CONFIGURED]");
    } else {
        println!("🔑 API key: [NOT SET - will check environment variables]");
    }
    
    Ok(())
}