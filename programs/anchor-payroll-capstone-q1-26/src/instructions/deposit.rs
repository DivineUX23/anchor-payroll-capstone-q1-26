use anchor_lang::prelude::*;
use anchor_spl::{associated_token::AssociatedToken, token_interface::{Mint, TokenAccount, TokenInterface}};

use crate::state::ProtocolVault;

#[derive(Accounts)]
#[instruction(seed: u64)]
pub struct Deposit<'info> {

    #[account(mut)]
    pub operator: Signer<'info>,

    #[account(mint::token_program = token_program)]
    pub usdc: InterfaceAccount<'info, Mint>,

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
        has_one = usdc,
        seeds = [b"protocol", operator.key().as_ref(), seed.to_le_bytes().as_ref()],
        bump
    )]
    pub protocol: Account<'info, ProtocolVault>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>
}

impl <'info>Deposit<'info> {
    pub fn init(&mut self) -> Result<()> {
        self.protocol.set_inner(ProtocolVault {
            safety_amount: 0,
            yield_amount: 0,
            global_rate: 0,
            liability: 0,
            liability_timestamp: 0,
        });
        Ok(())
    }
}