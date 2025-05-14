use anyhow::{Result, anyhow};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    message::Message,
    native_token::LAMPORTS_PER_SOL,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    signer::EncodableKey,
    system_instruction,
    transaction::Transaction,
};
use std::{collections::HashMap, str::FromStr, time::Duration};
use tokio::{fs, time::sleep};
use tonic::transport::ClientTlsConfig;
use yellowstone_grpc_client::GeyserGrpcClient;
use yellowstone_grpc_proto::geyser::{SubscribeRequest, SubscribeRequestFilterBlocks};

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    sender_keypair_file: String,
    recipient_address: String,
    amount_sol: f64,
}

pub async fn geyser(file: String) -> Result<()> {
    let config = load_config(&file).await?;

    // Create RPC client for transaction submission
    let rpc_url = dotenv::var("RPC_URL").expect("Missing solana rpc url");
    let rpc_client = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());

    let sender_keypair = Keypair::read_from_file(config.sender_keypair_file)
        .map_err(|e| anyhow!("failed to read sender keypair: {e:?}"))?;
    if rpc_client
        .get_account(&sender_keypair.pubkey())
        .await
        .is_err()
    {
        println!("sender account doesn't exist: {}", sender_keypair.pubkey());
        rpc_client
            .request_airdrop(&sender_keypair.pubkey(), LAMPORTS_PER_SOL / 100)
            .await?;
        println!("airdrop completed");
    }

    // Parse recipient pubkey
    let recipient = Pubkey::from_str(&config.recipient_address)?;
    if rpc_client.get_account(&recipient).await.is_err() {
        println!("recipient account doesn't exist: {}", recipient);
        rpc_client
            .request_airdrop(&recipient, LAMPORTS_PER_SOL / 100)
            .await?;
        println!("airdrop completed");
    }

    // Convert SOL amount to lamports
    let amount_lamports = (config.amount_sol * LAMPORTS_PER_SOL as f64) as u64;

    // Establish connection to Geyser GRPC
    let x_token = dotenv::var("GEYSER_X_TOKEN").expect("Missing geyser x token");

    let endpoint = dotenv::var("GEYSER_ENDPOINT").expect("Missing geyser endpoint");
    let tls_config = ClientTlsConfig::new().with_native_roots();
    let builder = GeyserGrpcClient::build_from_shared(endpoint)?
        .tls_config(tls_config)?
        .x_token(Some(x_token))?;
    let mut client = builder.connect().await?;

    // Set up block subscription
    let mut blocks = HashMap::new();
    blocks.insert(
        "client".to_string(),
        SubscribeRequestFilterBlocks::default(),
    );
    let subscribe_request = SubscribeRequest {
        blocks,
        ..Default::default()
    };
    let (mut _subscribe_tx, mut block_subscription) = client
        .subscribe_with_request(Some(subscribe_request))
        .await?;
    println!("Subscription set up successfully. Monitoring for new blocks...");

    // Monitor for new blocks
    while let Some(block_update) = block_subscription.next().await {
        match block_update {
            Ok(update) => {
                println!("New block detected: slot {}", update.created_at.unwrap());

                // Send transaction
                match send_sol_transaction(
                    &rpc_client,
                    &sender_keypair,
                    &recipient,
                    amount_lamports,
                )
                .await
                {
                    Ok(signature) => {
                        println!("Transaction sent successfully! Signature: {}", signature);

                        let balance = rpc_client.get_balance(&recipient).await.unwrap();
                        println!(
                            "Recipient balance: {}",
                            balance as f64 / LAMPORTS_PER_SOL as f64
                        );
                    }
                    Err(err) => eprintln!("Failed to send transaction: {}", err),
                }
            }
            Err(err) => {
                eprintln!("Error receiving block update: {}", err);
                // Try to reconnect after error
                sleep(Duration::from_secs(5)).await;
            }
        }
    }

    Ok(())
}

async fn load_config(path: &str) -> Result<Config> {
    let config: Config = serde_yaml::from_str(&fs::read_to_string(path).await?)?;
    Ok(config)
}

async fn send_sol_transaction(
    rpc_client: &RpcClient,
    sender: &Keypair,
    recipient: &Pubkey,
    amount_lamports: u64,
) -> Result<String> {
    // Create transfer instruction
    let instruction = system_instruction::transfer(&sender.pubkey(), recipient, amount_lamports);

    // Get recent blockhash
    let blockhash = rpc_client.get_latest_blockhash().await?;

    // Create and sign transaction
    let message = Message::new(&[instruction], Some(&sender.pubkey()));
    let transaction = Transaction::new(&[sender], message, blockhash);

    // Send transaction
    let signature = rpc_client
        .send_and_confirm_transaction(&transaction)
        .await?;

    Ok(signature.to_string())
}
