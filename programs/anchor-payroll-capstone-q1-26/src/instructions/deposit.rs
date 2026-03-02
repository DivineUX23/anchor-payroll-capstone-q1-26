use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction}, 
    program::invoke
};
use anchor_lang::solana_program::sysvar::instructions::ID as INSTRUCTIONS_ID;
use anchor_spl::{associated_token::AssociatedToken, token_interface::{Mint, TokenAccount, TokenInterface}};

use crate::state::{ProtocolVault};
use crate::utils::{get_sighash, KAMINO_PROGRAM_ID, USDC_MINT};

#[derive(Accounts)]
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
    pub operator_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        has_one = operator,
    )]
    pub protocol: Box<Account<'info, ProtocolVault>>,

    #[account(
        mut,
        associated_token::mint = usdc,
        associated_token::authority = operator,
        associated_token::token_program = token_program
    )]
    pub protocol_ktoken_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK:
    #[account(address = KAMINO_PROGRAM_ID)]
    pub kamino_program: AccountInfo<'info>,

    /// CHECK:
    #[account(mut)]
    pub reserve: AccountInfo<'info>,

    /// CHECK:
    pub lending_market: AccountInfo<'info>,
    /// CHECK:
    pub lending_market_authority: AccountInfo<'info>,

    #[account(address = USDC_MINT)]
    pub reserve_liquidity_mint: InterfaceAccount<'info, Mint>,

    /// CHECK: Kamino vault
    #[account(mut)]
    pub reserve_liquidity_supply: AccountInfo<'info>,

    #[account(mut)]
    pub reserve_collateral_mint: InterfaceAccount<'info, Mint>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,

    /// CHECK:
    #[account(address = INSTRUCTIONS_ID)]
    pub instruction_sysvar_account: AccountInfo<'info>,
}

impl <'info>Deposit<'info> {
    pub fn transfer(&mut self, deposit: u64) -> Result<()> {
        if deposit <= 0 {
            return Err(ProgramError::InvalidArgument.into());
        }

        let _ = self.protocol.update_liability();
        
        let ktoken_balance_before = self.protocol_ktoken_ata.amount;

        let mut data = get_sighash("deposit_reserve_liquidity").to_vec();
        data.extend_from_slice(&deposit.to_le_bytes());

        let accounts = vec![
            AccountMeta::new(self.operator.key(), true),
            AccountMeta::new(self.reserve.key(), false),
            AccountMeta::new_readonly(self.lending_market.key(), false),
            AccountMeta::new_readonly(self.lending_market_authority.key(), false),
            AccountMeta::new_readonly(self.reserve_liquidity_mint.key(), false),
            AccountMeta::new(self.reserve_liquidity_supply.key(), false),
            AccountMeta::new(self.reserve_collateral_mint.key(), false),
            AccountMeta::new(self.operator_ata.key(), false),
            AccountMeta::new(self.protocol_ktoken_ata.key(), false),
            AccountMeta::new_readonly(self.token_program.key(), false),
            AccountMeta::new_readonly(self.token_program.key(), false),
            AccountMeta::new_readonly(self.instruction_sysvar_account.key(), false),
        ];

        
        let ix = Instruction {
            program_id: self.kamino_program.key(),
            accounts,
            data
        };

        invoke(
            &ix,
            &[
                self.operator.to_account_info(),
                self.reserve.to_account_info(),
                self.lending_market.to_account_info(),
                self.lending_market_authority.to_account_info(),
                self.reserve_liquidity_mint.to_account_info(),
                self.reserve_liquidity_supply.to_account_info(),
                self.reserve_collateral_mint.to_account_info(),
                self.operator_ata.to_account_info(),
                self.protocol_ktoken_ata.to_account_info(),
                self.token_program.to_account_info(),
                self.token_program.to_account_info(),
                self.instruction_sysvar_account.to_account_info(),
            
            ]
        )?;

        self.protocol_ktoken_ata.reload()?;

        let ktoken_minted = self.protocol_ktoken_ata.amount
            .checked_sub(ktoken_balance_before)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        self.protocol.yield_amount = self.protocol.yield_amount
            .checked_add(ktoken_minted)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        Ok(())

    }

}