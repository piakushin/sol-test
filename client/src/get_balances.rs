use std::str::FromStr;

use anyhow::{Result, anyhow};
use futures::{TryStreamExt, stream::FuturesUnordered};
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{native_token::LAMPORTS_PER_SOL, pubkey::Pubkey};
use tokio::fs;

#[derive(Debug, Serialize, Deserialize)]
pub struct Balance {
    pubkey: Pubkey,
    balance: u64,
}

pub async fn get_balances(file: String) -> Result<()> {
    // Read config from YAML file
    let wallets: Vec<String> = serde_yaml::from_str(&fs::read_to_string(file).await?)?;

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
    for b in &balances {
        println!(
            "{} - {} SOL",
            b.pubkey,
            b.balance as f64 / LAMPORTS_PER_SOL as f64
        );
    }

    let output = serde_yaml::to_string(&balances)?;
    fs::write("balances.yaml", output).await?;

    Ok(())
}
