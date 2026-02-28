use anchor_lang::prelude::*;
use crate::state::{Reserve};
use solana_program::account_info::AccountInfo;

#[account]
#[derive(InitSpace)]
pub struct ProtocolVault {
    pub safety_amount: u64,
    pub yield_amount: u64,

    pub global_rate: u64,
    pub liability: u64,
    pub liability_timestamp: u64
}

impl ProtocolVault {
    pub fn update_global_liability(&mut self) -> Result<()> {
        let current_time = Clock::get().unwrap().unix_timestamp;
        let time_delta = current_time as u64 - self.liability_timestamp;

        self.liability += self.global_rate * time_delta;
        self.liability_timestamp = current_time as u64;

        Ok(())
    }


    pub fn update_protocol_vault(&mut self) -> u64 {
        let daily_burn_rate = self.global_rate * 3600 * 24;
        let two_days = 3600 * 48;
        
        let vault_target = (daily_burn_rate * two_days * 12) / 10;

        if self.safety_amount < vault_target {
            let to_safety = vault_target - self.safety_amount;
            to_safety
        } else {
            0
        }
    }

    pub fn calculate_liquid_capital(&self, k_info: &AccountInfo) -> Result<u64> {

        let reserve_data = k_info.try_borrow_data()
            .map_err(|_| ProgramError::InvalidAccountData)?;

        let reserve_size = size_of::<Reserve>();
        if reserve_data.len() < reserve_size + 8 {
            return Err(ProgramError::InvalidAccountData)?;
        }


        let k_reserve: &Reserve = bytemuck::try_from_bytes(&reserve_data[8..8 + reserve_size])
            .map_err(|_| ProgramError::InvalidAccountData)?;

        let wad: u128 = 1000000000000000000;

        let available_liq_sf = (k_reserve.liquidity.available_amount as u128)
            .checked_mul(wad)
            .ok_or(ProgramError::InvalidAccountData)?;

        let borrowed_liq_sf = (k_reserve.liquidity.borrowed_amount_sf[0] as u128) 
            | ((k_reserve.liquidity.borrowed_amount_sf[1] as u128) << 64);

        let protocol_fees_sf = (k_reserve.liquidity.accumulated_protocol_fees_sf[0] as u128) 
            | ((k_reserve.liquidity.accumulated_protocol_fees_sf[1] as u128) << 64);

        let total_pool_liq = available_liq_sf
            .checked_add(borrowed_liq_sf)
            .and_then(|x| x.checked_sub(protocol_fees_sf))
            .ok_or(ProgramError::ArithmeticOverflow)?
            / wad;

        let total_ktoken: u128 = k_reserve.collateral.mint_total_supply as u128;
        if total_ktoken == 0 {
            return Ok(self.safety_amount);
        }
        
        let cfo_ktoken_balance = self.yield_amount as u128;

        let cfo_usdc = cfo_ktoken_balance
            .checked_mul(total_pool_liq)
            .and_then(|v| v.checked_div(total_ktoken))
            .ok_or(ProgramError::ArithmeticOverflow)?;


        let total_assets = self.safety_amount
            .checked_add(cfo_usdc as u64)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        let current_time = Clock::get().unwrap().unix_timestamp;

        let time_delta = current_time.saturating_sub(self.liability_timestamp as i64);
        let new_liability = self.global_rate
            .checked_mul(time_delta as u64)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        let current_liability = self.liability
            .checked_add(new_liability)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        Ok(total_assets.saturating_sub(current_liability))

    }

}