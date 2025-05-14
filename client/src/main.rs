use std::fs;
use std::str::FromStr;

use anyhow::Result;
use anyhow::anyhow;
use clap::Parser;
use futures::stream::FuturesUnordered;
use futures::stream::TryStreamExt;
use serde::Deserialize;
use serde::Serialize;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;

mod transfer;

#[derive(Parser)]
enum CliCommands {
    GetBalances { file: String },
    Transfer { file: String },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Balance {
    pubkey: Pubkey,
    balance: u64,
}

async fn get_balances(file: String) -> Result<()> {
    // Read config from YAML file
    let wallets: Vec<String> = serde_yaml::from_str(&fs::read_to_string(file)?)?;

    // Connect to Solana network
    let rpc_url = dotenv::var("RPC_URL").expect("Missing solana rpc url");
    let rpc_client = RpcClient::new(rpc_url);

    // Retrieve and display balance for each wallet
    let handlers = FuturesUnordered::new();
    for wallet_address in &wallets {
        // Get the balance for the wallet
        let balance_fut = async {
            let pubkey = Pubkey::from_str(wallet_address)?;
            let balance = rpc_client
                .get_balance(&pubkey)
                .await
                .map_err(|e| anyhow!("failed to get balances: {e}"))?;
            Result::<_, anyhow::Error>::Ok(Balance { pubkey, balance })
        };
        handlers.push(balance_fut);
    }

    let balances = handlers.try_collect::<Vec<_>>().await?;

    let output = serde_yaml::to_string(&balances)?;
    fs::write("balances.yaml", output)?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv()?;
    let args = CliCommands::parse();

    match args {
        CliCommands::GetBalances { file } => get_balances(file).await?,
        CliCommands::Transfer { file } => transfer::transfer(file).await?,
    }

    return Ok(());
}
