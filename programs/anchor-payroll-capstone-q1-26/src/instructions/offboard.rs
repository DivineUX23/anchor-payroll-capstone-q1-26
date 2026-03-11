use anchor_lang::prelude::*;
use anchor_lang::solana_program::sysvar::instructions::ID as INSTRUCTIONS_ID;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction}, 
    program::invoke_signed
};
use anchor_spl::{associated_token::AssociatedToken, token_interface::{Mint, TokenAccount, TokenInterface, TransferChecked, transfer_checked}};
use crate::StaffAccount;
use crate::state::{ProtocolVault};
use crate::utils::{get_sighash, KAMINO_PROGRAM_ID, USDC_MINT};


#[derive(Accounts)]
pub struct StaffOffboard<'info> {

    #[account(mut)]
    pub operator: Signer<'info>,
    /// CHECK:
    pub staff: AccountInfo<'info>,

    #[account(mint::token_program = token_program)]
    pub usdc: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = usdc,
        associated_token::authority = staff,
        associated_token::token_program = token_program
    )]
    pub staff_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        //close = operator,
        constraint = staff_account.active == true @ ProgramError::InvalidAccountData
    )]
    pub staff_account: Account<'info, StaffAccount>,

    #[account(
        mut,
        has_one = operator,
    )]
    pub protocol: Account<'info, ProtocolVault>,
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
    pub protocol_ata: InterfaceAccount<'info, TokenAccount>,

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
    #[account(mut)]
    pub reserve: AccountInfo<'info>,
    //pub reserve: AccountLoader<'info, Reserve>,

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


impl <'info>StaffOffboard<'info> {

    pub fn claim_and_close(&mut self, bump: &StaffOffboardBumps) -> Result<()> {

        self.protocol.update_liability()?;
        let claimable_salary = self.staff_account.claimable_salary()?;

        let current_time = Clock::get()?.unix_timestamp as u64;
        self.staff_account.active = false;
        self.staff_account.time_ended = current_time;

        self.protocol.global_rate = self.protocol.global_rate
            .checked_sub(self.staff_account.rate)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        let binding = self.protocol.to_account_info().key();
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"authority",
            binding.as_ref(),
            &[bump.protocol_authority],
        ]];


        if claimable_salary > 0 {
            let mut usdc_received: u64 = 0;

            if claimable_salary > self.protocol.safety_amount {
                let debit_pool = claimable_salary.checked_sub(self.protocol.safety_amount)
                    .ok_or(ProgramError::ArithmeticOverflow)?;

                let ktoken_to_burn = self.protocol.ktoken_to_burn(debit_pool, self.protocol_ktoken_ata.amount, &self.reserve)?;

                usdc_received = self.k_withdrawal(ktoken_to_burn, signer_seeds)?;
            }
            
            let total_liq = self.protocol.safety_amount
                .checked_add(usdc_received)
                .ok_or(ProgramError::ArithmeticOverflow)?;

            if total_liq >= claimable_salary {

                self.protocol.safety_amount = total_liq
                    .checked_sub(claimable_salary)
                    .ok_or(ProgramError::ArithmeticOverflow)?;

                self.staff_account.total_claimed = self.staff_account.total_claimed
                    .checked_add(claimable_salary)
                    .ok_or(ProgramError::ArithmeticOverflow)?;

                self.protocol.liability = self.protocol.liability
                    .checked_sub(claimable_salary)
                    .ok_or(ProgramError::ArithmeticOverflow)?;

                self.debit_safety(claimable_salary, signer_seeds)?;

                //self.close_staff_account(signer_seeds)?;

            } else {
                self.protocol.safety_amount = total_liq;

                msg!("Protocol Illiquid. User offboarded but unpaid.")
            }

        }

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



    pub fn k_withdrawal(&mut self, ktoken: u64, signer_seeds: &[&[&[u8]]]) -> Result<u64> {
        
        if ktoken == 0 {
            return Err(ProgramError::InvalidArgument.into());
        }

        //self.protocol.update_liability()?;
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

/*
    pub fn close_staff_account(&mut self, signer_seeds: &[&[&[u8]]]) -> Result<()> {

        let close_accounts = CloseAccount{
            account: self.staff_account.to_account_info(),
            destination: self.operator.to_account_info(),
            authority: self.protocol_authority.to_account_info(),
        };

        let close_cpi = CpiContext::new_with_signer(
            self.token_program.to_account_info(), 
            close_accounts, 
            signer_seeds
        );

        close_account(close_cpi)
    }
*/

}