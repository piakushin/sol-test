use anyhow::Result;
use clap::Parser;

mod get_balances;
mod geyser;
mod transfer;

#[derive(Parser)]
enum CliCommands {
    GetBalances {
        file: String,
    },
    Transfer {
        file: String,
    },
    Geyser {
        #[clap(short, long, default_value_t = String::from("geyser.yaml"))]
        file: String,
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
    }
    Ok(())
}
