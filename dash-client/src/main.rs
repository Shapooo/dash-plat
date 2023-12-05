use dash_client::{client::Client, config::Config};

use anyhow::Result;
use clap::Arg;
use log::LevelFilter;
use simple_logger::SimpleLogger;

#[tokio::main]
async fn main() -> Result<()> {
    init_logger()?;
    let config = init_config().await?;
    Client::new(config)?.run().await?;
    Ok(())
}

fn init_logger() -> Result<()> {
    SimpleLogger::new()
        .with_level(LevelFilter::Debug)
        .env()
        .init()?;
    Ok(())
}

async fn init_config() -> Result<Config> {
    let args = clap::command!()
        .arg(
            Arg::new("config")
                .short('c')
                .long("config_path")
                .action(clap::ArgAction::Set)
                .help(
                    "set config file path, defaults to `client_config.yaml' \
                     in the same directory as dash-client binary",
                ),
        )
        .get_matches();
    if let Some(path) = args.get_one::<String>("config") {
        Config::from_path(path).await
    } else {
        Config::new().await
    }
}
