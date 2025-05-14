use anyhow::Result;
use clap::Parser;

mod get_balances;
mod geyser;
mod prepare;
mod transfer;

#[derive(Parser)]
enum CliCommands {
    GetBalances {
        #[clap(short, long, default_value_t = String::from("wallets.yaml"))]
        file: String,
    },
    Transfer {
        #[clap(short, long, default_value_t = String::from("transfer.yaml"))]
        file: String,
    },
    Geyser {
        #[clap(short, long, default_value_t = String::from("geyser.yaml"))]
        file: String,
    },
    Prepare {
        #[clap(short, long, default_value_t = String::from("wallets.yaml"))]
        balances_config: String,
        #[clap(short, long, default_value_t = String::from("transfer.yaml"))]
        transfer_config: String,
        #[clap(short, long, default_value_t = String::from("geyser.yaml"))]
        geyser_config: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv()?;
    let args = CliCommands::parse();

    match args {
        CliCommands::GetBalances { file } => get_balances::get_balances(file).await?,
        CliCommands::Transfer { file } => transfer::transfer(file).await?,
        CliCommands::Geyser { file } => geyser::geyser(file).await?,
        CliCommands::Prepare {
            balances_config,
            transfer_config,
            geyser_config,
        } => prepare::prepare(&balances_config, transfer_config, geyser_config).await?,
    }
    Ok(())
}
