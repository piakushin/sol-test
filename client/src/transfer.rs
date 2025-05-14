use std::fs;

use anyhow::Result;
use futures::{TryStreamExt, stream::FuturesUnordered};
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig, message::Message, pubkey::Pubkey, signature::Keypair,
    signer::Signer, system_instruction, transaction::Transaction,
};
use tokio::time::Instant;

#[derive(Debug, Serialize, Deserialize)]
pub struct WalletsPair {
    from_pk: String,
    to: Pubkey,
    amount_lamp: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct TransferResult {
    signature: String,
    source: String,
    destination: String,
    status: String,
    processing_time_ms: u64,
}

pub async fn transfer(file: String) -> Result<()> {
    // Read config file
    let wallets: Vec<WalletsPair> = serde_yaml::from_str(&fs::read_to_string(file)?)?;

    // Connect to Solana network
    let rpc_url = dotenv::var("RPC_URL").expect("Missing solana rpc url");

    // Perform transfers
    let results = batch_transfer(wallets, rpc_url).await?;

    // Print results
    print_transfer_results(&results);

    Ok(())
}

async fn batch_transfer(
    wallets_pairs: Vec<WalletsPair>,
    rpc_url: String,
) -> Result<Vec<TransferResult>> {
    let commitment_config = CommitmentConfig::confirmed();
    let rpc_client = RpcClient::new_with_commitment(rpc_url, commitment_config);

    let handlers = FuturesUnordered::new();
    for wallets in wallets_pairs {
        handlers.push(single_transfer(commitment_config, &rpc_client, wallets));
    }
    let output = handlers.try_collect().await?;
    Ok(output)
}

async fn single_transfer(
    commitment_config: CommitmentConfig,
    rpc_client: &RpcClient,
    wallets: WalletsPair,
) -> Result<TransferResult, anyhow::Error> {
    let source_keypair = Keypair::from_base58_string(&wallets.from_pk);
    let recent_blockhash = rpc_client.get_latest_blockhash().await?;
    let instruction =
        system_instruction::transfer(&source_keypair.pubkey(), &wallets.to, wallets.amount_lamp);
    let message = Message::new(&[instruction], Some(&source_keypair.pubkey()));
    let transaction = Transaction::new(&[&source_keypair], message, recent_blockhash);

    // Send tx and measure completion time.
    let start_time = Instant::now();
    let signature = rpc_client
        .send_and_confirm_transaction(&transaction)
        .await?;
    let elapsed = start_time.elapsed().as_millis() as u64;

    let status = rpc_client
        .get_signature_status_with_commitment(&signature, commitment_config)
        .await?
        .expect("assuming unreachable")
        .map(|_| "success")
        .unwrap_or("failure");
    let result = TransferResult {
        signature: signature.to_string(),
        source: source_keypair.pubkey().to_string(),
        destination: wallets.to.to_string(),
        status: status.to_string(),
        processing_time_ms: elapsed,
    };
    Ok(result)
}

fn print_transfer_results(results: &[TransferResult]) {
    println!("Transfer Results:");
    println!("{:<64} {:<10} {:<10}", "Signature", "Status", "Time (ms)");
    println!("{}", "-".repeat(86));

    let mut success_count = 0;
    let mut failed_count = 0;
    let mut total_time = 0;

    for result in results {
        println!(
            "{:<64} {:<10} {:<10}",
            result.signature, result.status, result.processing_time_ms
        );

        if result.status == "success" {
            success_count += 1;
        } else {
            failed_count += 1;
        }

        total_time += result.processing_time_ms;
    }

    println!("\nSummary:");
    println!("Total transfers: {}", results.len());
    println!("Successful: {}", success_count);
    println!("Failed: {}", failed_count);
    println!(
        "Average processing time: {} ms",
        total_time / results.len() as u64
    );
    println!("Total processing time: {} ms", total_time);
}
