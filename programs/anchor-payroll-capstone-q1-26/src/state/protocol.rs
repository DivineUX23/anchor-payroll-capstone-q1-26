use anchor_lang::prelude::*;
use crate::state::{Reserve};

#[account]
#[derive(InitSpace)]
pub struct ProtocolVault {
    //pub seed: u64,
    pub operator: Pubkey,
    pub safety_amount: u64,
    pub yield_amount: u64,
    pub global_rate: u64,
    pub liability: u64,
    pub liability_timestamp: u64
}

impl ProtocolVault {
    pub fn update_liability(&mut self) -> Result<()> {
        let current_time = Clock::get()?.unix_timestamp as u64;
        //let time_delta = current_time as u64 - self.liability_timestamp;
        let time_delta = current_time
            .saturating_sub(self.liability_timestamp);

        if time_delta > 0 {
            let total_amount = self.global_rate
                .checked_mul(time_delta)
                .ok_or(ProgramError::ArithmeticOverflow)?;

            //self.liability += self.global_rate * time_delta;
            self.liability = self.liability
                .checked_add(total_amount)
                .ok_or(ProgramError::ArithmeticOverflow)?;

            self.liability_timestamp = current_time;
        }

        Ok(())
    }


    pub fn update_protocol(&mut self) -> u64 {
        let daily_burn_rate = self.global_rate * 3600 * 24;
        let vault_target = (daily_burn_rate * 2 * 12) / 10;

        if self.safety_amount < vault_target {
            //let to_safety = vault_target - self.safety_amount;
            let to_safety = vault_target
                .saturating_sub(self.safety_amount);

        msg!("Update protocol to safety: {}", to_safety);

            to_safety
        } else {
            0
        }
    }

    

    pub fn calculate_k_pool(&self, k_info: &AccountInfo) -> Result<(u128, u128)> {
        
        let reserve_data = k_info.try_borrow_data()
            .map_err(|_| ProgramError::InvalidAccountData)?;

        let reserve_size = std::mem::size_of::<Reserve>();
        if reserve_data.len() < reserve_size + 8 {
            return Err(ProgramError::InvalidAccountData)?;
        }

        let k_reserve: &Reserve = bytemuck::try_from_bytes(&reserve_data[8..8 + reserve_size])
            .map_err(|_| ProgramError::InvalidAccountData)?;

        //let k_reserve = k_info.load()?;
        //let wad: u128 = 1_000_000_000_000_000_000;
        let wad: u128 = 1u128 << 60;

        let available_sf = (k_reserve.liquidity.available_amount as u128)
            .checked_mul(wad)
            .ok_or(ProgramError::InvalidAccountData)?;

        let borrowed_sf = (k_reserve.liquidity.borrowed_amount_sf[0] as u128) 
            | ((k_reserve.liquidity.borrowed_amount_sf[1] as u128) << 64);

        let protocol_fees_sf = (k_reserve.liquidity.accumulated_protocol_fees_sf[0] as u128) 
            | ((k_reserve.liquidity.accumulated_protocol_fees_sf[1] as u128) << 64);

        let referrer_fees_sf = (k_reserve.liquidity.accumulated_referrer_fees_sf[0] as u128) 
            | ((k_reserve.liquidity.accumulated_referrer_fees_sf[1] as u128) << 64);

        let pending_referrer_fees_sf = (k_reserve.liquidity.pending_referrer_fees_sf[0] as u128) 
            | ((k_reserve.liquidity.pending_referrer_fees_sf[1] as u128) << 64);

        let fees_sf = protocol_fees_sf
            .checked_add(referrer_fees_sf)
            .and_then(|x| x.checked_add(pending_referrer_fees_sf))
            .ok_or(ProgramError::ArithmeticOverflow)?;

        let total_pool_usdc = available_sf
            .checked_add(borrowed_sf)
            .and_then(|x| x.checked_sub(fees_sf))
            .ok_or(ProgramError::ArithmeticOverflow)?
            / wad;

        let total_ktoken: u128 = k_reserve.collateral.mint_total_supply as u128;
        
        msg!("Initial Total USDC: {}", total_pool_usdc);
        msg!("Initial Total K token: {}", total_ktoken);

        Ok((total_pool_usdc, total_ktoken))
    }



    pub fn calculate_total_assets(&self, k_info: &AccountInfo) -> Result<u64> {

        let (total_pool_usdc,  total_ktoken) = self.calculate_k_pool(k_info)?;

        if total_ktoken == 0 {
            return Ok(self.safety_amount);
        }
        
        let cfo_ktoken_balance = self.yield_amount as u128;

        let cfo_usdc = cfo_ktoken_balance
            .checked_mul(total_pool_usdc)
            .and_then(|v| v.checked_div(total_ktoken))
            .ok_or(ProgramError::ArithmeticOverflow)?;


        let total_assets = self.safety_amount
            .checked_add(cfo_usdc as u64)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        let current_time = Clock::get()?.unix_timestamp;

        let time_delta = current_time.saturating_sub(self.liability_timestamp as i64);
        let new_liability = self.global_rate
            .checked_mul(time_delta as u64)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        let current_liability = self.liability
            .checked_add(new_liability)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        Ok(total_assets.saturating_sub(current_liability))

    }


    pub fn ktoken_to_burn(&self, debit_pool: u64, p_k_amount: u64, reserve: &AccountInfo) -> Result<u64> {
        
        let (total_pool_usdc,  total_ktoken) = self.calculate_k_pool(reserve)?;

        let ktoken_to_burn = {

            let numertor = (debit_pool as u128)
                .checked_mul(total_ktoken)
                .ok_or(ProgramError::ArithmeticOverflow)?;     

            let ktoken_to_burn = numertor
                .checked_add(total_pool_usdc)
                .and_then(|x| x.checked_sub(1))
                .and_then(|x| x.checked_div(total_pool_usdc))
                .ok_or(ProgramError::ArithmeticOverflow)?
                as u64;

            const LIQ_OFFSET: usize = 224;
            let reserve_data = reserve.try_borrow_data()?;
            let mut reserve_byte = [0u8; 8];
            reserve_byte.copy_from_slice(&reserve_data[LIQ_OFFSET..LIQ_OFFSET + 8]);
            let max_liq = u64::from_le_bytes(reserve_byte);


            let max_liq_k = (max_liq as u128)
                .checked_mul(total_ktoken)
                .and_then(|x| x.checked_div(total_pool_usdc))
                .ok_or(ProgramError::ArithmeticOverflow)?
                as u64;

            ktoken_to_burn
                .min(p_k_amount)
                .min(max_liq_k)
        };

        Ok(ktoken_to_burn)

    }

}