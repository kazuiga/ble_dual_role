mod central;
mod constants;
mod peripheral;

use std::process::exit;

use anyhow::Result;
use btleplug::api::BDAddr;
use clap::Parser;
use env_logger::{Builder, Env};
use log::{error, info};

use crate::{central::handle_central, peripheral::handle_peripheral};

#[derive(Debug, Parser)]
#[command(author, version, long_about = None, arg_required_else_help = true)]
struct Cli {
    /// 対象の Bluetooth デバイスのアドレス
    address: BDAddr,
}

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> Result<()> {
    Builder::from_env(Env::default().default_filter_or("info")).init();

    let cli = Cli::parse();

    info!("ターゲットデバイスのアドレス: {:?}", cli.address);

    tokio::spawn(async move {
        if let Err(e) = handle_central(&cli.address).await {
            error!("{:?}", e);
            exit(1);
        }
    });

    handle_peripheral().await?;

    Ok(())
}
