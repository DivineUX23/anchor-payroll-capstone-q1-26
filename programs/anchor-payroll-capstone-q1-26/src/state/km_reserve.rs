use anchor_lang::prelude::*;
use crate::state::{LastUpdate, ReserveLiquidity, ReserveCollateral, ReserveConfig};

//static_assertions::const_assert_eq!(RESERVE_SIZE, std::mem::size_of::<Reserve>());
//static_assertions::const_assert_eq!(0, std::mem::size_of::<Reserve>() % 8);
#[derive(PartialEq)]//, Derivative)]
//#[derivative(Debug)]
#[account(zero_copy)]
//#[zero_copy]
#[repr(C)]
pub struct Reserve {
    pub version: u64,
    
    pub last_update: LastUpdate,
    
    pub lending_market: Pubkey,
    
    pub farm_collateral: Pubkey,
    
    pub farm_debt: Pubkey,
    
    pub liquidity: ReserveLiquidity,
    
    //#[derivative(Debug = "ignore")]
    pub reserve_liquidity_padding: [u64; 150],
    
    pub collateral: ReserveCollateral,
    
    //#[derivative(Debug = "ignore")]
    pub reserve_collateral_padding: [u64; 150],
    
    pub config: ReserveConfig,
    
    //#[derivative(Debug = "ignore")]
    pub config_padding: [u64; 114],
    
    pub borrowed_amount_outside_elevation_group: u64,
    
    pub borrowed_amounts_against_this_reserve_in_elevation_groups: [u64; 32],
    
    //#[derivative(Debug = "ignore")]
    pub padding: [u64; 207],
}