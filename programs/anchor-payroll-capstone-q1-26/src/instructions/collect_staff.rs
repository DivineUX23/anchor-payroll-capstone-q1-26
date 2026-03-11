use anchor_lang::prelude::*;
use crate::state::StaffAccount;
use crate::state::ProtocolVault;

#[derive(Accounts)]
pub struct CollectStaff<'info> {
    #[account(mut)]
    pub operator: Signer<'info>,

    #[account(
        has_one = operator @ ProgramError::InvalidAccountData
    )]
    pub protocol: Account<'info, ProtocolVault>,

    #[account(
        mut,
        close = operator,
        constraint = staff_account.active == false @ ProgramError::InvalidAccountData
    )]
    pub staff_account: Account<'info, StaffAccount>,

}

impl <'info> CollectStaff <'info> {
    pub fn collect_close(&mut self) -> Result<()> {

        let unpaid = self.staff_account.claimable_salary()?;

        if unpaid > 0 {
            msg!("Systemic Risk: Account holds frozen unpaid yields.");
            return Err(ProgramError::InvalidAccountData.into());
        }

        msg!("Staff account physically closed. Rent reclaimed.");
        Ok(())
    }
}