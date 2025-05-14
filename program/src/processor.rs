use solana_program::{
    account_info::{AccountInfo, next_account_info},
    entrypoint::ProgramResult,
    msg,
    program::invoke,
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction,
};

pub struct Processor;

impl Processor {
    pub fn initialize_account(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let user = next_account_info(account_info_iter)?;
        let user_deposit_account = next_account_info(account_info_iter)?;
        let system_program = next_account_info(account_info_iter)?;

        // Check if the account is already initialized
        if user_deposit_account.data_len() > 0 {
            return Err(ProgramError::AccountAlreadyInitialized);
        }

        // Verify account ownership
        if user_deposit_account.owner != program_id {
            return Err(ProgramError::IncorrectProgramId);
        }

        // Initialize the account with zero balance
        let mut data = user_deposit_account.try_borrow_mut_data()?;
        data[0..8].copy_from_slice(&0u64.to_le_bytes());

        msg!("Account initialized");
        Ok(())
    }

    pub fn deposit(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let user = next_account_info(account_info_iter)?;
        let user_deposit_account = next_account_info(account_info_iter)?;
        let system_program = next_account_info(account_info_iter)?;

        // Verify account ownership
        if user_deposit_account.owner != program_id {
            return Err(ProgramError::IncorrectProgramId);
        }

        // Get the amount of lamports to deposit
        let amount = **user.lamports.borrow();

        // Transfer lamports from user to deposit account
        invoke(
            &system_instruction::transfer(user.key, user_deposit_account.key, amount),
            &[
                user.clone(),
                user_deposit_account.clone(),
                system_program.clone(),
            ],
        )?;

        // Update the user's balance
        let mut data = user_deposit_account.try_borrow_mut_data()?;
        let current_balance = u64::from_le_bytes(data[0..8].try_into().unwrap());
        let new_balance = current_balance
            .checked_add(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        data[0..8].copy_from_slice(&new_balance.to_le_bytes());

        msg!("Deposit successful");
        Ok(())
    }

    pub fn withdraw(program_id: &Pubkey, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let user = next_account_info(account_info_iter)?;
        let user_deposit_account = next_account_info(account_info_iter)?;

        // Verify account ownership
        if user_deposit_account.owner != program_id {
            return Err(ProgramError::IncorrectProgramId);
        }

        // Check if the requester is the owner
        if !user.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Get the current balance
        let mut data = user_deposit_account.try_borrow_mut_data()?;
        let current_balance = u64::from_le_bytes(data[0..8].try_into().unwrap());

        // Check if the user has enough balance
        if amount > current_balance {
            return Err(ProgramError::InsufficientFunds);
        }

        // Transfer lamports from deposit account to user
        **user_deposit_account.lamports.borrow_mut() = user_deposit_account
            .lamports()
            .checked_sub(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        **user.lamports.borrow_mut() = user
            .lamports()
            .checked_add(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        // Update the user's balance
        let new_balance = current_balance
            .checked_sub(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        data[0..8].copy_from_slice(&new_balance.to_le_bytes());

        msg!("Withdrawal successful");
        Ok(())
    }
}
