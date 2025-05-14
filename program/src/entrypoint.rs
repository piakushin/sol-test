use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, msg, pubkey::Pubkey};

use crate::{instruction::DepositInstruction, processor::Processor};

solana_program::entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = DepositInstruction::unpack(instruction_data)?;

    match instruction {
        DepositInstruction::Initialize => {
            msg!("Instruction: Initialize");
            Processor::initialize_account(program_id, accounts)
        }
        DepositInstruction::Deposit => {
            msg!("Instruction: Deposit");
            Processor::deposit(program_id, accounts)
        }
        DepositInstruction::Withdraw { amount } => {
            msg!("Instruction: Withdraw");
            Processor::withdraw(program_id, accounts, amount)
        }
    }
}
