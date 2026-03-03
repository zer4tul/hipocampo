//! Hipocampo CLI

use clap::Parser;

#[derive(Parser)]
#[command(name = "hipocampo")]
#[command(about = "Agent-first unified memory backend", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    /// Index markdown files
    Index {
        /// Paths to index
        #[arg(short, long)]
        paths: Vec<String>,
    },
    /// Search memories
    Search {
        /// Search query
        query: String,
        /// Maximum results
        #[arg(short = 'n', long, default_value = "10")]
        limit: usize,
    },
    /// Show system status
    Status,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Index { paths: _ } => {
            println!("Indexing... (not implemented yet)");
        }
        Commands::Search { query, limit: _ } => {
            println!("Searching for: {} (not implemented yet)", query);
        }
        Commands::Status => {
            println!("Hipocampo v0.1.0");
            println!("Status: Ready");
        }
    }

    Ok(())
}
