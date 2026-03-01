use anchor_lang::prelude::*;
use anchor_lang::solana_program::sysvar::instructions::ID as INSTRUCTIONS_ID;
use anchor_spl::{associated_token::AssociatedToken, token_interface::{Mint, TokenAccount, TokenInterface, TransferChecked, transfer_checked}};

pub const KAMINO_PROGRAM_ID: Pubkey = pubkey!("KLend2g3cPENfacJ1B3121X7A62BwY75q25w1d8nLZk");
pub const USDC_MINT: Pubkey = pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
pub const BOUNTY_AMOUNT: u64 = 100_000;
pub const PLATFORM_TAX: u64 = 50;

use crate::StaffAccount;
use crate::state::{ProtocolVault};

#[derive(Accounts)]
#[instruction(seed: u64)]
pub struct StaffClaim<'info> {

    #[account(mut)]
    pub operator: Signer<'info>,


    pub staff: AccountInfo<'info>,

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
        close = operator,
    )]
    pub staff_account: Account<'info, StaffAccount>,


    #[account(
        mut,
        associated_token::mint = usdc,
        associated_token::authority = operator,
        associated_token::token_program = token_program
    )]
    pub operator_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        close = operator,
        has_one = operator,
    )]
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

    #[account(address = INSTRUCTIONS_ID)]
    pub instruction_sysvar_account: AccountInfo<'info>,
}


impl <'info>StaffClaim<'info> {

    pub fn withdraw(&mut self, amount: u64, protocol_bump: u8) -> Result<()> {

        let _ = self.protocol.update_liability();

        let time = Clock::get().unwrap().unix_timestamp as u64;

        let time_passed = time.checked_sub(self.staff_account.time_started)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        let claimable_salary = self.staff_account.rate.checked_mul(time_passed)
            .and_then(|x| x.checked_div(self.staff_account.total_claimed))
            .ok_or(ProgramError::ArithmeticOverflow)?;


        if claimable_salary == 0 {
            return Err(ProgramError::InsufficientFunds.into());
        }


        if self.protocol.safety_amount < claimable_salary {
            return Err(ProgramError::InsufficientFunds.into())
        }

        let binding = self.protocol.to_account_info().key();
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"authority",
            binding.as_ref(),
            &[protocol_bump],
        ]];


        let _ = self.debit_safety(amount, signer_seeds);

        self.protocol.safety_amount = self.protocol.safety_amount
            .checked_sub(amount)
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