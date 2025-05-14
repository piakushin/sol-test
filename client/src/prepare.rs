use anyhow::Result;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{native_token::LAMPORTS_PER_SOL, signature::Keypair, signer::Signer};
use tokio::fs;

pub async fn prepare(
    balances_config: &str,
    transfer_config: String,
    geyser_config: String,
) -> Result<()> {
    prepare_balances_config(balances_config).await?;
    prepare_transfer_config(transfer_config).await?;
    prepare_geyser_config(geyser_config).await
}

async fn prepare_balances_config(config_file: &str) -> Result<()> {
    if fs::try_exists(config_file).await? {
        println!("Balances file already exists. Delete it to regenerate.");
        return Ok(());
    }

    let rpc_url = dotenv::var("RPC_URL").expect("Missing solana rpc url");
    let rpc_client = RpcClient::new(rpc_url);

    let wallets: Vec<String> = (0..500)
        .map(|i| {
            let keypair = Keypair::new();
            rpc_client
                .request_airdrop(&keypair.pubkey(), LAMPORTS_PER_SOL / (1000 - i))
                .expect("failed to request airdrop");
            println!("Wallet {i}/500 funded");
            keypair.pubkey().to_string()
        })
        .collect();
    let output = serde_yaml::to_string(&wallets)?;
    fs::write(config_file, output).await?;

    Ok(())
}

async fn prepare_geyser_config(config_file: String) -> Result<()> {
    todo!()
}

async fn prepare_transfer_config(config_file: String) -> Result<()> {
    todo!()
}
