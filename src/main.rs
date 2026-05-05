use anyhow::Result;
use clap::{Parser, Subcommand};

mod adapter;
mod binding;
mod cmd;
mod config;
mod skill;

#[derive(Parser)]
#[command(name = "manas", version, about = "Ops surface for the manas ecosystem")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Check health of mcpjungle and all upstream subsystems
    Health,
    /// Boot a rich session (full Tool Group binding, memory, handoff)
    Warm,
    /// Session shutdown: store observations, write handoff, revoke binding
    Done,
    /// Between-session maintenance: consolidate observations into mental models
    Reflect,
    /// Show active sessions, bindings, and lock state
    Status,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Health => cmd::health::run().await,
        Command::Warm => cmd::warm::run().await,
        Command::Done => cmd::done::run().await,
        Command::Reflect => cmd::reflect::run().await,
        Command::Status => cmd::status::run().await,
    }
}
