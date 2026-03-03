//! Hipocampo CLI

use clap::{Parser, Subcommand};
use hipocampo::{
    embedding::{openai::OpenAIModel, EmbeddingProvider, NoopEmbedding},
    indexer::MarkdownIndexer,
    memory::{ListFilter, MemoryCategory, SearchOptions},
    storage::sqlite::SqliteBackend,
    Memory,
};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Parser)]
#[command(name = "hipocampo")]
#[command(about = "Agent-first unified memory backend", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Workspace directory (defaults to current directory)
    #[arg(short, long, global = true)]
    workspace: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    /// Index markdown files
    Index {
        /// Use OpenAI embeddings (requires OPENAI_API_KEY)
        #[arg(short, long)]
        openai: bool,

        /// OpenAI model to use
        #[arg(long, default_value = "text-embedding-3-small")]
        model: String,
    },

    /// Search memories
    Search {
        /// Search query
        query: String,

        /// Maximum results
        #[arg(short = 'n', long, default_value = "10")]
        limit: usize,

        /// Filter by session
        #[arg(short, long)]
        session: Option<String>,

        /// Hybrid search (vector + keyword)
        #[arg(long, default_value = "true")]
        hybrid: bool,
    },

    /// List all memories
    List {
        /// Filter by category
        #[arg(short, long)]
        category: Option<String>,

        /// Limit results
        #[arg(short = 'n', long)]
        limit: Option<usize>,
    },

    /// Show system status
    Status,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let cli = Cli::parse();
    let workspace = cli.workspace.unwrap_or_else(|| std::env::current_dir().unwrap());

    match cli.command {
        Commands::Index { openai, model } => {
            index_command(&workspace, openai, &model).await?;
        }
        Commands::Search {
            query,
            limit,
            session,
            hybrid,
        } => {
            search_command(&workspace, &query, limit, session, hybrid).await?;
        }
        Commands::List { category, limit } => {
            list_command(&workspace, category, limit).await?;
        }
        Commands::Status => {
            status_command(&workspace).await?;
        }
    }

    Ok(())
}

async fn index_command(workspace: &PathBuf, use_openai: bool, model_name: &str) -> anyhow::Result<()> {
    println!("Indexing workspace: {}", workspace.display());

    let embedder: Box<dyn EmbeddingProvider> = if use_openai {
        let model = match model_name {
            "text-embedding-3-large" => OpenAIModel::TextEmbedding3Large,
            "text-embedding-ada-002" => OpenAIModel::TextEmbeddingAda002,
            _ => OpenAIModel::TextEmbedding3Small,
        };
        Box::new(hipocampo::embedding::openai::OpenAIEmbedding::from_env(model)?)
    } else {
        Box::new(NoopEmbedding)
    };

    let backend = Arc::new(NoopEmbedding);
    let memory = SqliteBackend::new(workspace, backend)?;
    let indexer = MarkdownIndexer::new(memory, embedder, workspace.clone());

    let stats = indexer.index_workspace().await?;

    println!("\n✅ Indexing complete:");
    println!("  MEMORY.md: {} chunks", stats.memory_md);
    println!("  Daily files: {} ({} chunks)", stats.daily_files, stats.daily_chunks);
    println!("  Total memories: {}", stats.total);

    Ok(())
}

async fn search_command(
    workspace: &PathBuf,
    query: &str,
    limit: usize,
    session: Option<String>,
    hybrid: bool,
) -> anyhow::Result<()> {
    let memory = SqliteBackend::new(workspace, Arc::new(NoopEmbedding))?;

    let opts = SearchOptions {
        limit,
        session_id: session,
        hybrid,
        ..Default::default()
    };

    let results = memory.search(query, opts).await?;

    println!("Found {} results for '{}':\n", results.len(), query);

    for (i, entry) in results.iter().enumerate() {
        println!("{}. {} (score: {:.3})", i + 1, entry.key, entry.score.unwrap_or(0.0));
        println!("   {}", entry.content.lines().next().unwrap_or(""));
        println!();
    }

    Ok(())
}

async fn list_command(
    workspace: &PathBuf,
    category: Option<String>,
    limit: Option<usize>,
) -> anyhow::Result<()> {
    let memory = SqliteBackend::new(workspace, Arc::new(NoopEmbedding))?;

    let cat = category.and_then(|c| match c.as_str() {
        "core" => Some(MemoryCategory::Core),
        "daily" => Some(MemoryCategory::Daily),
        "conversation" => Some(MemoryCategory::Conversation),
        _ => None,
    });

    let filter = ListFilter {
        category: cat,
        limit,
        ..Default::default()
    };

    let entries = memory.list(filter).await?;

    println!("Total memories: {}\n", entries.len());

    for entry in entries {
        println!("- [{}] {} ({} bytes)", entry.category, entry.key, entry.content.len());
    }

    Ok(())
}

async fn status_command(workspace: &PathBuf) -> anyhow::Result<()> {
    let memory = SqliteBackend::new(workspace, Arc::new(NoopEmbedding))?;

    println!("Hipocampo v0.1.0");
    println!("Workspace: {}", workspace.display());
    println!("Backend: {}", memory.name());
    println!("Total memories: {}", memory.count().await?);
    println!("Health: {}", if memory.health_check().await { "✅ OK" } else { "❌ ERROR" });

    Ok(())
}
