use anchor_lang::prelude::*;
use anchor_spl::{associated_token::AssociatedToken, token_interface::{Mint, TokenAccount, TokenInterface}};
use crate::{ProtocolVault, state::StaffAccount};

#[derive(Accounts)]
#[instruction(seed: u64)]
pub struct StaffInit<'info> {

    #[account(mut)]
    pub operator: Signer<'info>,
    /// CHECK:
    pub staff: AccountInfo<'info>,

    #[account(mint::token_program = token_program)]
    pub usdc: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        has_one = operator,
    )]
    pub protocol: Account<'info, ProtocolVault>,

    #[account(
        init,
        payer = operator,
        seeds = [b"staff", staff.key().as_ref(), seed.to_le_bytes().as_ref()],
        space = StaffAccount::DISCRIMINATOR.len() + StaffAccount::INIT_SPACE,
        bump,
    )]
    pub staff_account: Account<'info, StaffAccount>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>
}

impl <'info>StaffInit<'info> {
    pub fn init(&mut self, annualized_salary: u64) -> Result<()> {

        let rate_sec = annualized_salary
            .checked_div(31_557_600 as u64)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        self.staff_account.set_inner(StaffAccount {
            active: true,
            rate: rate_sec,
            total_claimed: 0,
            time_started: Clock::get().unwrap().unix_timestamp as u64,
        });

        self.protocol.global_rate = self.protocol.global_rate
            .checked_add(rate_sec)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        self.protocol.update_liability()?;

        Ok(())
    }
}