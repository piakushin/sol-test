use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::program_error::ProgramError;

#[derive(Debug, BorshDeserialize, BorshSerialize)]
pub enum DepositInstruction {
    Initialize,
    Deposit,
    Withdraw { amount: u64 },
}

impl DepositInstruction {
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (&tag, rest) = input
            .split_first()
            .ok_or(ProgramError::InvalidInstructionData)?;

        Ok(match tag {
            0 => Self::Initialize,
            1 => Self::Deposit,
            2 => {
                if rest.len() < 8 {
                    return Err(ProgramError::InvalidInstructionData);
                }
                let amount = u64::from_le_bytes(rest[..8].try_into().unwrap());
                Self::Withdraw { amount }
            }
            _ => return Err(ProgramError::InvalidInstructionData),
        })
    }
}
