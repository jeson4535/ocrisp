use clap::{Parser, Subcommand};

#[derive(Subcommand)]
pub enum Commands {
    /// Start MCP server
    Mcp,

    /// Start a REST server
    Serve,

    /// Wipe the database, for any reason
    Wipe,
    
    /// Minimal query-chat, recommended to debug your search
    QueryChat
}

#[derive(Parser)]
#[command(
    name = "ocrisp",
    version = "0.1",
    about = "One-Click RAG Implementation, Simple and Portable"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

pub async fn run_cli(command: Commands) {
    match command {
        Commands::Mcp => {
            let _ = crate::api::mcp::run_mcp().await;
        }
        Commands::Serve => {
            let _ = crate::api::rest::run_rest();
        }
        Commands::Wipe => {
            println!("Deleting database");
            match wipe() {
                Ok(_) => {
                    println!("Success")
                }
                Err(e) => {
                    println!("An error ocurred: {}", e)
                }
            }
        }
        Commands::QueryChat => {
            let _ = crate::api::query_chat::run_query_chat().await;
        }
    }
}

fn wipe() -> std::io::Result<()> {
    // Delete "storage" folder if it exists
    let storage_path = std::path::Path::new("storage");
    if storage_path.exists() {
        std::fs::remove_dir_all(storage_path)?;
    }

    // Delete "snapshots" folder if it exists
    let snapshots_path = std::path::Path::new("snapshots");
    if snapshots_path.exists() {
        std::fs::remove_dir_all(snapshots_path)?;
    }

    // Delete ".qdrant-initialized" file if it exists
    let init_file = std::path::Path::new(".qdrant-initialized");
    if init_file.exists() {
        std::fs::remove_file(init_file)?;
    }

    Ok(())
}
