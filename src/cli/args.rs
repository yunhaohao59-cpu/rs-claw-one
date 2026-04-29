use clap::Args;

#[derive(Args, Debug, Clone)]
pub struct CliArgs {
    #[arg(long, default_value = "18789")]
    pub port: u16,

    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,

    #[arg(long)]
    pub config: Option<String>,
}
