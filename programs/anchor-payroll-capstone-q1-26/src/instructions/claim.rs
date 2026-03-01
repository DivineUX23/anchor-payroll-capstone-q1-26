use anchor_lang::prelude::*;
use anchor_spl::{associated_token::AssociatedToken, token_interface::{Mint, TokenAccount, TokenInterface, TransferChecked, transfer_checked}};
use crate::StaffAccount;
use crate::state::{ProtocolVault};

#[derive(Accounts)]
pub struct StaffClaim<'info> {

    #[account(mut)]
    pub staff: Signer<'info>,

    #[account(mint::token_program = token_program)]
    pub usdc: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = usdc,
        associated_token::authority = staff,
        associated_token::token_program = token_program
    )]
    pub staff_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        constraint = staff_account.active == true @ ProgramError::InvalidAccountData
    )]
    pub staff_account: Account<'info, StaffAccount>,


    #[account(mut)]
    pub protocol: Account<'info, ProtocolVault>,

    #[account(
        seeds = [b"authority", protocol.key().as_ref()],
        bump
    )]
    pub protocol_authority: AccountInfo<'info>,

    #[account(mut,
        associated_token::mint = usdc,
        associated_token::authority = protocol_authority,
        associated_token::token_program = token_program
    )]
    pub protocol_ata: InterfaceAccount<'info, TokenAccount>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,
}


impl <'info>StaffClaim<'info> {

    pub fn claim(&mut self, bump: &StaffClaimBumps) -> Result<()> {

        let _ = self.protocol.update_liability();

        let current_time = Clock::get().unwrap().unix_timestamp as u64;

        let time_passed = current_time
            .checked_sub(self.staff_account.time_started)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        let claimable_salary = self.staff_account.rate
            .checked_mul(time_passed)
            .and_then(|x| x.checked_sub(self.staff_account.total_claimed))
            .ok_or(ProgramError::ArithmeticOverflow)?;


        if claimable_salary == 0 {
            msg!("No salary earned yet.");
            return Err(ProgramError::InsufficientFunds.into());
        }


        if self.protocol.safety_amount < claimable_salary {
            msg!("Protocol treasury is illiquid. Await Keeper Rebalance");
            return Err(ProgramError::InsufficientFunds.into())
        }

        let binding = self.protocol.to_account_info().key();
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"authority",
            binding.as_ref(),
            &[bump.protocol_authority],
        ]];


        let _ = self.debit_safety(claimable_salary, signer_seeds);

        self.protocol.safety_amount = self.protocol.safety_amount
            .checked_sub(claimable_salary)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        self.staff_account.total_claimed = self.staff_account.total_claimed
            .checked_add(claimable_salary)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        self.protocol.liability = self.protocol.liability
            .checked_sub(claimable_salary)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        Ok(())

    }


    pub fn debit_safety(&mut self, amount: u64, signer_seeds: &[&[&[u8]]]) -> Result<()> {

        let transfer_accounts = TransferChecked{
            from: self.protocol_ata.to_account_info(),
            mint: self.usdc.to_account_info(),
            to: self.staff_ata.to_account_info(),
            authority: self.protocol_authority.to_account_info(),
        };

        let withdraw_cpi = CpiContext::new_with_signer(
            self.token_program.to_account_info(), 
            transfer_accounts, 
            signer_seeds
        );

        transfer_checked(withdraw_cpi, amount, self.usdc.decimals)?;
        Ok(())
    }
}