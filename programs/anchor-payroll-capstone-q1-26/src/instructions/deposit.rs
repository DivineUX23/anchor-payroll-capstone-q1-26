use anchor_lang::prelude::*;
use anchor_lang::solana_program::sysvar::instructions::ID as INSTRUCTIONS_ID;
use anchor_spl::{associated_token::AssociatedToken, token_interface::{Mint, TokenAccount, TokenInterface}};
use kamino_lending::cpi::accounts::DepositReserveLiquidity;
use kamino_lending::program::KaminoLending;

use crate::state::{ProtocolVault};

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

    #[account(
        mut,
        associated_token::mint = usdc,
        associated_token::authority = operator,
        associated_token::token_program = token_program
    )]
    pub protocol_ktoken_ata: Account<'info, TokenAccount>,


    pub kamino_program: Program<'info, KaminoLending>,
    #[account(mut)]
    pub reserve: AccountInfo<'info>,

    pub lending_market: AccountInfo<'info>,
    pub lending_market_authority: AccountInfo<'info>,

    pub reserve_liquidity_mint: AccountInfo<'info, Mint>,

    #[account(mut)]
    pub reserve_liquidity_supply: AccountInfo<'info, TokenAccount>,

    #[account(mut)]
    pub reserve_collateral_mint: AccountInfo<'info, Mint>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,

    #[account(address = INSTRUCTIONS_ID)]
    pub instruction_sysvar_account: AccountInfo<'info>,
}

impl <'info>Deposit<'info> {
    pub fn transfer(&mut self, deposit: u64) -> Result<()> {
        if deposit <= 0 {
            return Err(ProgramError::InvalidArgument.into());
        }

        let ktoken_balance_before = self.protocol_ktoken_ata.amount;

        let cpi_accounts = DepositReserveLiquidity {
            owner: self.operator.to_account_info(),
            reserve: self.reserve.to_account_info(),
            lending_market: self.lending_market.to_account_info(),
            lending_market_authority: self.lending_market_authority.to_account_info(),
            reserve_liquidity_mint: self.reserve_liquidity_mint.to_account_info(),
            reserve_liquidity_supply: self.reserve_liquidity_supply.to_account_info(),
            reserve_collateral_mint: self.reserve_collateral_mint.to_account_info(),
            user_source_liquidity: self.operator_ata.to_account_info(),
            user_destination_collateral: self.protocol_ktoken_ata.to_account_info(),
            collateral_token_program: self.token_program.to_account_info(),
            liquidity_token_program: self.token_program.to_account_info(),
            instruction_sysvar_account: self.instruction_sysvar_account.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(
            self.kamino_program.to_account_info(),
            cpi_accounts
        );

        kamino_lending::cpi::deposit_reserve_liquidity(cpi_ctx, deposit)?;

        self.protocol_ktoken_ata.reload()?;

        let ktoken_balance_after = self.protocol_ktoken_ata.amount;

        let ktoken_minted = ktoken_balance_after
            .checked_sub(ktoken_balance_before)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        self.protocol.yield_amount = self.protocol.yield_amount
            .checked_add(ktoken_minted)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        Ok(())

    }

}