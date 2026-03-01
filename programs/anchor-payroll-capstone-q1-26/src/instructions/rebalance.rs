use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction}, 
    program::invoke_signed
};
use anchor_lang::solana_program::sysvar::instructions::ID as INSTRUCTIONS_ID;
use anchor_spl::{associated_token::AssociatedToken, token_interface::{Mint, TokenAccount, TokenInterface, TransferChecked, transfer_checked}};

pub const KAMINO_PROGRAM_ID: Pubkey = pubkey!("KLend2g3cPENfacJ1B3121X7A62BwY75q25w1d8nLZk");
pub const USDC_MINT: Pubkey = pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
pub const BOUNTY_AMOUNT: u64 = 100_000;
pub const PLATFORM_TAX: u64 = 50;

use crate::{get_sighash, state::{ProtocolVault}};

#[derive(Accounts)]
#[instruction(seed: u64)]
pub struct Rebalance<'info> {

    #[account(mut)]
    pub keeper: Signer<'info>,

    pub operator: AccountInfo<'info>,

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
        associated_token::mint = usdc,
        associated_token::authority = keeper,
        associated_token::token_program = token_program
    )]
    pub keeper_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        close = operator,
        seeds = [b"protocol", operator.key().as_ref(), seed.to_le_bytes().as_ref()],
        bump
    )]
    pub protocol: Account<'info, ProtocolVault>,


    #[account(mut,
        associated_token::mint = usdc,
        associated_token::authority = protocol,
        associated_token::token_program = token_program
    )]
    pub protocol_ata: InterfaceAccount<'info, TokenAccount>,


    #[account(
        seeds = [b"authority", protocol.key().as_ref()],
        bump
    )]
    pub protocol_authority: AccountInfo<'info>,

    #[account(
        mut,
        associated_token::mint = usdc,
        associated_token::authority = operator,
        associated_token::token_program = token_program
    )]
    pub protocol_ktoken_ata: InterfaceAccount<'info, TokenAccount>,

    // add address = PLATFORM_TREASURY
    #[account(mut)]
    pub platform_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(address = KAMINO_PROGRAM_ID)]
    pub kamino_program: AccountInfo<'info>,
    
    #[account(mut)]
    pub reserve: AccountInfo<'info>,

    pub lending_market: AccountInfo<'info>,
    pub lending_market_authority: AccountInfo<'info>,

    #[account(address = USDC_MINT)]
    pub reserve_liquidity_mint: InterfaceAccount<'info, Mint>,

    #[account(mut)]
    pub reserve_liquidity_supply: AccountInfo<'info>,

    #[account(mut)]
    pub reserve_collateral_mint: InterfaceAccount<'info, Mint>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,

    #[account(address = INSTRUCTIONS_ID)]
    pub instruction_sysvar_account: AccountInfo<'info>,
}


impl <'info>Rebalance<'info> {

    pub fn rebalance_pay(&mut self, protocol_bump: u8) -> Result<()> {

        let required_usdc = self.protocol.update_protocol_vault();

        if required_usdc == 0 {
            msg!("Warning: Protocol is already balanced.");
            return Ok(()); 
        }

        let platform_tax: u64 = required_usdc
            .checked_mul(PLATFORM_TAX)
            .and_then(|x| x.checked_div(10000))
            .ok_or(ProgramError::ArithmeticOverflow)?;

        let total_ddt = BOUNTY_AMOUNT
            .checked_add(platform_tax)            
            .ok_or(ProgramError::ArithmeticOverflow)?;


        let (total_pool_usdc,  total_ktoken) = self.protocol.calculate_k_pool(&self.reserve)?;
        
        if (total_pool_usdc as u64) < total_ddt {
            msg!("Warning: Lending pool illiquid. Extraction deferred.");
            return Ok(()); 
        }

        let ktoken_to_burn = (required_usdc as u128)
            .checked_mul(total_ktoken)
            .and_then(|x| x.checked_div(total_pool_usdc))
            .ok_or(ProgramError::ArithmeticOverflow)?
            as u64;

        let binding = self.protocol.to_account_info().key();
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"authority",
            binding.as_ref(),
            &[protocol_bump],]
        ];

        // Execute CPI to Kamino to withdraw `required_usdc`
        let usdc_recieved = self.k_withdrawal(ktoken_to_burn, signer_seeds)?;
        
        if usdc_recieved < total_ddt {
            return Err(ProgramError::InsufficientFunds.into());
        }

        // Send bounty
        let bounty_accounts = TransferChecked {
            from: self.protocol_ata.to_account_info(),
            mint: self.usdc.to_account_info(),
            to: self.keeper_ata.to_account_info(),
            authority: self.protocol.to_account_info(),
        };
        let keeper_cpi = CpiContext::new_with_signer(
            self.token_program.to_account_info(), 
            bounty_accounts,
            signer_seeds
        );
        transfer_checked(keeper_cpi, BOUNTY_AMOUNT, self.usdc.decimals)?;

        //platform Tax
        let tax_accounts = TransferChecked {
            from: self.protocol_ata.to_account_info(),
            mint: self.usdc.to_account_info(),
            to: self.platform_ata.to_account_info(),
            authority: self.protocol.to_account_info(),
        };
        let tax_cpi = CpiContext::new_with_signer(
            self.token_program.to_account_info(), 
            tax_accounts,
            signer_seeds
        );
        transfer_checked(tax_cpi, platform_tax, self.usdc.decimals)?;

        self.protocol.safety_amount = self.protocol.safety_amount
            .checked_add(usdc_recieved)
            .and_then(|x| x.checked_sub(total_ddt))
            .ok_or(ProgramError::ArithmeticOverflow)?;
        //self.protocol.yield_amount -= (required_usdc + total_ddt);

        Ok(())

    }




    pub fn k_withdrawal(&mut self, ktoken: u64, signer_seeds: &[&[&[u8]]]) -> Result<(u64)> {
        
        if ktoken == 0 {
            return Err(ProgramError::InvalidArgument.into());
        }

        self.protocol.update_global_liability();
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
            AccountMeta::new(self.token_program.key(), false),
            AccountMeta::new_readonly(self.token_program.key(), false),
            AccountMeta::new_readonly(self.token_program.key(), false),
            AccountMeta::new_readonly(self.instruction_sysvar_account.key(), false),
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
                self.instruction_sysvar_account.to_account_info(),
            ],
            signer_seeds,
        )?;

        self.protocol_ata.reload()?;

        let usdc_received = self.protocol_ata.amount
            .checked_sub(balance_before)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        self.protocol.yield_amount = self.protocol.yield_amount
            .checked_sub(usdc_received)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        Ok(usdc_received)

    }
}