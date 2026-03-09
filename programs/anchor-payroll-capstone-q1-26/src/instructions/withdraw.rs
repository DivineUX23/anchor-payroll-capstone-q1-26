use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction}, 
    program::invoke_signed
};
use anchor_lang::solana_program::sysvar::instructions::ID as INSTRUCTIONS_ID;
use anchor_spl::{associated_token::AssociatedToken, token_interface::{Mint, TokenAccount, TokenInterface, TransferChecked, transfer_checked}};

use crate::state::{ProtocolVault, Reserve};
use crate::utils::{get_sighash, KAMINO_PROGRAM_ID, USDC_MINT};



#[derive(Accounts)]
#[instruction(seed: u64)]
pub struct CFOWithdraw<'info> {

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
        has_one = operator,
    )]
    pub protocol: Box<Account<'info, ProtocolVault>>,

    /// CHECK:
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
    pub protocol_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = reserve_collateral_mint,
        associated_token::authority = protocol_authority,
        associated_token::token_program = token_program
    )]
    pub protocol_ktoken_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK:
    #[account(address = KAMINO_PROGRAM_ID)]
    pub kamino_program: AccountInfo<'info>,
    /// CHECK:
    #[account(
        mut, 
        owner = kamino_program.key()
    )]
    //pub reserve: AccountInfo<'info>,
    pub reserve: AccountLoader<'info, Reserve>,

    /// CHECK:
    pub lending_market: AccountInfo<'info>,
    /// CHECK:
    pub lending_market_authority: AccountInfo<'info>,

    #[account(address = USDC_MINT)]
    pub reserve_liquidity_mint: InterfaceAccount<'info, Mint>,
    /// CHECK:
    #[account(mut)]
    pub reserve_liquidity_supply: AccountInfo<'info>,

    #[account(mut)]
    pub reserve_collateral_mint: InterfaceAccount<'info, Mint>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,
    /// CHECK:
    #[account(address = INSTRUCTIONS_ID)]
    pub instruction_sysvar: AccountInfo<'info>,
}


impl <'info>CFOWithdraw<'info> {

    pub fn withdraw(&mut self, amount: u64, bump: &CFOWithdrawBumps) -> Result<()> {

        let available_usdc = self.protocol.calculate_total_assets(&self.reserve)?;
        if available_usdc < amount {
            return Err(ProgramError::InsufficientFunds.into());
        };

        let binding = self.protocol.to_account_info().key();
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"authority",
            binding.as_ref(),
            &[bump.protocol_authority],
        ]];

        if amount > self.protocol.safety_amount {

            let debit_pool = amount.checked_sub(self.protocol.safety_amount)
                .ok_or(ProgramError::ArithmeticOverflow)?;

            let (total_pool_usdc,  total_ktoken) = self.protocol.calculate_k_pool(&self.reserve)?;

            let ktoken_to_burn = (debit_pool as u128)
                .checked_mul(total_ktoken)
                .and_then(|x| x.checked_div(total_pool_usdc))
                .ok_or(ProgramError::ArithmeticOverflow)?
                as u64;

            let ktoken_to_burn = ktoken_to_burn.min(self.protocol_ktoken_ata.amount);

            let usdc_received = self.k_withdrawal(ktoken_to_burn, signer_seeds)?;
            
            self.protocol.safety_amount = self.protocol.safety_amount
                .checked_add(usdc_received)
                .ok_or(ProgramError::ArithmeticOverflow)?;

        }
        let _ = self.debit_safety(amount, signer_seeds)?;

        self.protocol.safety_amount = self.protocol.safety_amount
            .checked_sub(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        //self.protocol.yield_amount -= (required_usdc + total_ddt);

        Ok(())

    }


    pub fn debit_safety(&mut self, amount: u64, signer_seeds: &[&[&[u8]]]) -> Result<()> {

        let transfer_accounts = TransferChecked{
            from: self.protocol_ata.to_account_info(),
            mint: self.usdc.to_account_info(),
            to: self.operator_ata.to_account_info(),
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


    pub fn k_withdrawal(&mut self, ktoken: u64, signer_seeds: &[&[&[u8]]]) -> Result<u64> {
        
        if ktoken == 0 {
            return Ok(0);
        }

        let _ = self.protocol.update_liability()?;
        let balance_before = self.protocol_ata.amount;

        let mut data = get_sighash("redeem_reserve_collateral").to_vec();
        data.extend_from_slice(&ktoken.to_le_bytes());

        let accounts = vec![
            AccountMeta::new_readonly(self.protocol_authority.key(), true),
            AccountMeta::new_readonly(self.lending_market.key(), false),
            AccountMeta::new(self.reserve.key(), false),
            AccountMeta::new_readonly(self.lending_market_authority.key(), false),
            AccountMeta::new_readonly(self.reserve_liquidity_mint.key(), false),
            AccountMeta::new(self.reserve_collateral_mint.key(), false),
            AccountMeta::new(self.reserve_liquidity_supply.key(), false),
            AccountMeta::new(self.protocol_ktoken_ata.key(), false),
            AccountMeta::new(self.protocol_ata.key(), false),
            AccountMeta::new_readonly(self.token_program.key(), false),
            AccountMeta::new_readonly(self.token_program.key(), false),
            AccountMeta::new_readonly(self.instruction_sysvar.key(), false),
        ];

        
        let ix = Instruction {
            program_id: self.kamino_program.key(),
            accounts,
            data
        };

        invoke_signed(
            &ix,
            &[
                self.protocol_authority.to_account_info(),
                self.lending_market.to_account_info(),
                self.reserve.to_account_info(),
                self.lending_market_authority.to_account_info(),
                self.reserve_liquidity_mint.to_account_info(),
                self.reserve_collateral_mint.to_account_info(),
                self.reserve_liquidity_supply.to_account_info(),
                self.protocol_ktoken_ata.to_account_info(),
                self.protocol_ata.to_account_info(),
                self.token_program.to_account_info(),
                self.token_program.to_account_info(),
                self.instruction_sysvar.to_account_info(),
            ],
            signer_seeds,
        )?;

        self.protocol_ata.reload()?;

        let usdc_received = self.protocol_ata.amount
            .checked_sub(balance_before)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        self.protocol.yield_amount = self.protocol.yield_amount
            .checked_sub(ktoken)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        Ok(usdc_received)

    }
}