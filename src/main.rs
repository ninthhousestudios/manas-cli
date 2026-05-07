use anyhow::Result;
use clap::{Parser, Subcommand};

mod adapter;
mod binding;
mod cmd;
mod config;
mod hub;
mod skill;

#[derive(Parser)]
#[command(name = "manas", version, about = "Ops surface for the manas ecosystem")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Check health of manas subsystems (chitta, yojana, sangha)
    Health,
    /// Boot a rich session (memory, handoff, task context)
    Warm {
        /// Harness to launch: claude-code, codex, gemini, opencode
        #[arg(default_value = "claude-code")]
        harness: String,
    },
    /// Session shutdown: store observations, write handoff, revoke binding
    Done,
    /// Between-session maintenance: consolidate observations into mental models
    Reflect,
    /// Show active sessions, bindings, and lock state
    Status,
    /// Run the manas HTTP MCP server (composed tools: wake_up, ingest)
    Serve {
        #[arg(short, long, default_value = "3000")]
        port: u16,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Health => cmd::health::run().await,
        Command::Warm { harness } => cmd::warm::run(&harness).await,
        Command::Done => cmd::done::run().await,
        Command::Reflect => cmd::reflect::run().await,
        Command::Status => cmd::status::run().await,
        Command::Serve { port } => cmd::serve::run(port).await,
    }
}
