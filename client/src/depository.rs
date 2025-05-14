use anyhow::{Result, bail};

use borsh::{BorshDeserialize, BorshSerialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_instruction,
};
use solana_sdk::{
    bpf_loader_upgradeable,
    commitment_config::CommitmentConfig,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use std::str::FromStr;
use std::{env, fs, io, path::Path};

// Instructions recognized by the program
#[derive(BorshSerialize, BorshDeserialize, Debug)]
enum DepositInstruction {
    Initialize,
    Deposit,
    Withdraw { amount: u64 },
}

pub async fn depository() -> Result<()> {
    // Connect to the cluster
    let rpc_url = dotenv::var("RPC_URL").expect("Missing solana rpc url");
    let client = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());

    // Load or create payer keypair
    let payer = load_or_create_keypair()?;
    println!("Using payer: {}", payer.pubkey());

    // Check and maybe fund payer account
    let balance = client.get_balance(&payer.pubkey()).await?;
    println!("Payer balance: {} SOL", balance as f64 / 1_000_000_000.0);

    if balance < 1_000_000_000 {
        println!(
            "Payer account has insufficient funds. Please fund the account using solana CLI or an airdrop."
        );
        println!("For example: solana airdrop 2 {}", payer.pubkey());
        // Alternatively, you could automatically request an airdrop:
        // let sig = client.request_airdrop(&payer.pubkey(), 1_000_000_000)?;
        // client.confirm_transaction(&sig)?;
        return Ok(());
    }

    // Load or deploy the program
    let program_id = deploy_program_if_needed(&client, &payer).await?;
    println!("Using program ID: {program_id}");

    // Derive PDA for this user
    let (pda, _) = Pubkey::find_program_address(&[payer.pubkey().as_ref()], &program_id);
    println!("Derived PDA: {pda}");

    // Menu for interacting with the program
    loop {
        println!("\nDeposit Program Client");
        println!("1. Initialize account");
        println!("2. Deposit SOL");
        println!("3. Withdraw SOL");
        println!("4. Check balance");
        println!("5. Exit");
        println!("Choose an option (1-5):");

        let mut choice = String::new();
        io::stdin().read_line(&mut choice)?;
        let choice = choice.trim();

        match choice {
            "1" => initialize_account(&client, &payer, &program_id, pda).await?,
            "2" => deposit_sol(&client, &payer, &program_id, pda).await?,
            "3" => withdraw_sol(&client, &payer, &program_id, pda).await?,
            "4" => check_balance(&client, pda).await?,
            "5" => break,
            _ => println!("Invalid choice, please try again"),
        }
    }

    Ok(())
}

fn load_or_create_keypair() -> Result<Keypair> {
    let keypair_path = "dep_test_account.json";

    if Path::new(&keypair_path).exists() {
        println!("Loading keypair from {keypair_path}");
        let keypair_bytes = fs::read(keypair_path)?;
        let keypair_str = String::from_utf8(keypair_bytes)?;
        let keypair_vec: Vec<u8> = serde_json::from_str(&keypair_str)?;
        return Ok(Keypair::from_bytes(&keypair_vec)?);
    }

    // Create a new keypair
    println!("Creating new keypair");
    let keypair = Keypair::new();

    // Save the keypair
    let keypair_bytes = keypair.to_bytes();
    let keypair_json = serde_json::to_string(&keypair_bytes.to_vec())?;
    fs::write(keypair_path, keypair_json)?;
    println!("Keypair saved to {keypair_path}");

    Ok(keypair)
}

async fn deploy_program_if_needed(client: &RpcClient, payer: &Keypair) -> Result<Pubkey> {
    // Check if we have a saved program id
    let home_dir = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let program_id_path = format!("{home_dir}/.config/solana/deposit_program_id.txt");

    if Path::new(&program_id_path).exists() {
        let program_id_str = fs::read_to_string(&program_id_path)?;
        let program_id = Pubkey::from_str(program_id_str.trim())?;

        // Check if program exists on chain
        match client.get_account(&program_id).await {
            Ok(_) => {
                println!("Program already deployed at: {program_id}");
                return Ok(program_id);
            }
            Err(_) => {
                println!("Saved program ID exists but program not found on chain. Will deploy.");
            }
        }
    }

    // Program needs to be deployed
    println!("Deploying program...");

    // Usually you would compile the program first or ensure it's already compiled
    let program_path = "target/deploy/program.so";
    if !Path::new(program_path).exists() {
        bail!("Program binary not found. Please compile the program first with 'cargo build-sbf'");
    }

    // Read the program ELF
    let program_data = fs::read(program_path)?;

    // Create a new keypair for the program
    let program_keypair = Keypair::new();
    let program_id = program_keypair.pubkey();

    // Calculate required space
    let program_len = program_data.len();

    // Calculate minimum balance required for the program account
    let lamports = client
        .get_minimum_balance_for_rent_exemption(program_len)
        .await?;

    // Create the program account
    let create_account_instr = system_instruction::create_account(
        &payer.pubkey(),
        &program_id,
        lamports,
        program_len as u64,
        &bpf_loader_upgradeable::id(),
    );

    let transaction = Transaction::new_signed_with_payer(
        &[create_account_instr],
        Some(&payer.pubkey()),
        &[payer, &program_keypair],
        client.get_latest_blockhash().await?,
    );

    client.send_and_confirm_transaction(&transaction).await?;
    println!("Created program account");

    // Write program data to the account
    // Note: In a real deployment, you would use BPF loader to load the program
    // This is a simplified example - in practice, you'd use the solana CLI
    println!("For a real deployment, use the Solana CLI:");
    println!("solana program deploy {program_path}");

    // Save the program ID for future use
    fs::write(&program_id_path, program_id.to_string())?;
    println!("Program ID saved to {program_id_path}");

    bail!("Deploy program and restart")
}

async fn initialize_account(
    client: &RpcClient,
    payer: &Keypair,
    program_id: &Pubkey,
    pda: Pubkey,
) -> Result<()> {
    println!("Initializing account...");

    // Create instruction data for Initialize
    let instruction_data = DepositInstruction::Initialize;

    // Create the instruction
    let instruction = Instruction::new_with_borsh(
        *program_id,
        &instruction_data,
        vec![
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new(pda, false),
            AccountMeta::new_readonly(solana_program::system_program::id(), false),
        ],
    );

    // Create and send the transaction
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[payer],
        client.get_latest_blockhash().await?,
    );

    let signature = client.send_and_confirm_transaction(&transaction).await?;
    println!("Account initialized! Transaction signature: {signature}");
    Ok(())
}

async fn deposit_sol(
    client: &RpcClient,
    payer: &Keypair,
    program_id: &Pubkey,
    pda: Pubkey,
) -> Result<()> {
    println!("Enter amount to deposit in SOL:");
    let mut amount_str = String::new();
    io::stdin().read_line(&mut amount_str)?;
    let amount_sol = amount_str.trim().parse::<f64>()?;
    let amount_lamports = (amount_sol * 1_000_000_000.0) as u64;

    println!(
        "Depositing {} SOL ({} lamports)...",
        amount_sol, amount_lamports
    );

    // Create instruction data for Deposit
    let instruction_data = DepositInstruction::Deposit;

    // First transfer SOL to the program account
    let transfer_instruction = system_instruction::transfer(&payer.pubkey(), &pda, amount_lamports);

    // Then update the balance in the account's data
    let deposit_instruction = Instruction::new_with_borsh(
        *program_id,
        &instruction_data,
        vec![
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new(pda, false),
            AccountMeta::new_readonly(solana_program::system_program::id(), false),
        ],
    );

    // Create and send the transaction
    let transaction = Transaction::new_signed_with_payer(
        &[transfer_instruction, deposit_instruction],
        Some(&payer.pubkey()),
        &[payer],
        client.get_latest_blockhash().await?,
    );

    let signature = client.send_and_confirm_transaction(&transaction).await?;
    println!("Deposit successful! Transaction signature: {}", signature);
    Ok(())
}

async fn withdraw_sol(
    client: &RpcClient,
    payer: &Keypair,
    program_id: &Pubkey,
    pda: Pubkey,
) -> Result<()> {
    println!("Enter amount to withdraw in SOL:");
    let mut amount_str = String::new();
    io::stdin().read_line(&mut amount_str)?;
    let amount_sol = amount_str.trim().parse::<f64>()?;
    let amount_lamports = (amount_sol * 1_000_000_000.0) as u64;

    println!(
        "Withdrawing {} SOL ({} lamports)...",
        amount_sol, amount_lamports
    );

    // Create instruction data for Withdraw
    let mut instruction_data = vec![2]; // Instruction index for Withdraw
    instruction_data.extend_from_slice(&amount_lamports.to_le_bytes());

    // Create the instruction
    let instruction = Instruction::new_with_bytes(
        *program_id,
        &instruction_data,
        vec![
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new(pda, false),
            AccountMeta::new_readonly(solana_program::system_program::id(), false),
        ],
    );

    // Create and send the transaction
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[payer],
        client.get_latest_blockhash().await?,
    );

    let signature = client.send_and_confirm_transaction(&transaction).await?;
    println!(
        "Withdrawal successful! Transaction signature: {}",
        signature
    );
    Ok(())
}

async fn check_balance(client: &RpcClient, pda: Pubkey) -> Result<()> {
    println!("Checking account balance...");

    // Get account info - lamports and data
    match client.get_account(&pda).await {
        Ok(account) => {
            let lamports_balance = account.lamports;
            println!(
                "Account lamports: {} ({} SOL)",
                lamports_balance,
                lamports_balance as f64 / 1_000_000_000.0
            );

            // Try to read the stored balance from account data
            if account.data.len() >= 8 {
                let stored_balance = u64::from_le_bytes(account.data[0..8].try_into().unwrap());
                println!(
                    "Stored balance: {} ({} SOL)",
                    stored_balance,
                    stored_balance as f64 / 1_000_000_000.0
                );
            } else {
                println!("Account doesn't have valid data yet. Please initialize it first.");
            }
        }
        Err(_) => {
            println!("Account not found. Please initialize it first.");
        }
    }

    Ok(())
}
