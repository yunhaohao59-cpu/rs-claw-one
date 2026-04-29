use clap::Parser;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

mod cli;
mod gateway;
mod agent;
mod memory;
mod skill;
mod tools;
mod model;
mod storage;
mod config;
mod context;

#[derive(Parser)]
#[command(name = "rs-claw", version, about = "RS-Claw — AI-powered computer control")]
struct Args {
    #[command(subcommand)]
    command: Option<cli::Command>,

    #[arg(long, default_value = "info")]
    log_level: String,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("info")))
        .with(fmt::layer())
        .init();

    let args = Args::parse();

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    rt.block_on(async move {
        match args.command {
            Some(cmd) => cli::run_command(cmd).await,
            None => cli::run_repl().await,
        }
    })
}
