use anchor_lang::prelude::*;
use crate::state::{BigFractionBytes, ReserveFees, BorrowRateCurve, TokenInfo, WithdrawalCaps};


//#[derive( Debug)]
#[derive(Debug, PartialEq, Eq)]
#[zero_copy]
#[repr(C)]
pub struct ReserveLiquidity {
    pub mint_pubkey: Pubkey,
    pub supply_vault: Pubkey,
    pub fee_vault: Pubkey,
    pub available_amount: u64,
    //pub borrowed_amount_sf: u128,
    //pub market_price_sf: u128,

    pub borrowed_amount_sf: [u64; 2],
    pub market_price_sf: [u64; 2],

    pub market_price_last_updated_ts: u64,
    pub mint_decimals: u64,
    pub deposit_limit_crossed_timestamp: u64,
    pub borrow_limit_crossed_timestamp: u64,
    pub cumulative_borrow_rate_bsf: BigFractionBytes,
    
    
    //pub accumulated_protocol_fees_sf: u128,
    //pub accumulated_referrer_fees_sf: u128,
    //pub pending_referrer_fees_sf: u128,
    //pub absolute_referral_rate_sf: u128,
    
    pub accumulated_protocol_fees_sf: [u64; 2],
    pub accumulated_referrer_fees_sf: [u64; 2],
    pub pending_referrer_fees_sf: [u64; 2],
    pub absolute_referral_rate_sf: [u64; 2],
    
    pub token_program: Pubkey,
    pub padding2: [u64; 51],
    //pub padding3: [u128; 32],
    pub padding3: [u64; 64],
}

//#[derive( Clone, Debug)]
#[derive(Debug, PartialEq, Eq)]
#[zero_copy]
#[repr(C)]
pub struct ReserveCollateral {
    pub mint_pubkey: Pubkey,
    pub mint_total_supply: u64,
    pub supply_vault: Pubkey,
    //pub padding1: [u128; 32],
    //pub padding2: [u128; 32],
    pub padding1: [u64; 64],
    pub padding2: [u64; 64],
}


#[derive(PartialEq, Eq, Default)] //Derivative
//#[derivative(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[zero_copy]
#[repr(C)]
pub struct ReserveConfig {
    pub status: u8,
    pub padding_deprecated_asset_tier: u8,
    pub host_fixed_interest_rate_bps: u16,
    pub min_deleveraging_bonus_bps: u16,

    #[cfg_attr(feature = "serde", serde(with = "serde_bool_u8"))]
    pub block_ctoken_usage: u8,

    #[cfg_attr(feature = "serde", serde(skip_serializing, default))]
    //#[derivative(Debug = "ignore")]
    pub reserved1: [u8; 6],

    pub protocol_order_execution_fee_pct: u8,
    pub protocol_take_rate_pct: u8,
    pub protocol_liquidation_fee_pct: u8,
    pub loan_to_value_pct: u8,
    pub liquidation_threshold_pct: u8,
    pub min_liquidation_bonus_bps: u16,
    pub max_liquidation_bonus_bps: u16,
    pub bad_debt_liquidation_bonus_bps: u16,
    pub deleveraging_margin_call_period_secs: u64,
    pub deleveraging_threshold_decrease_bps_per_day: u64,
    pub fees: ReserveFees,
    pub borrow_rate_curve: BorrowRateCurve,
    pub borrow_factor_pct: u64,
    pub deposit_limit: u64,
    pub borrow_limit: u64,
    pub token_info: TokenInfo,
    pub deposit_withdrawal_cap: WithdrawalCaps,
    pub debt_withdrawal_cap: WithdrawalCaps,
    pub elevation_groups: [u8; 20],
    pub disable_usage_as_coll_outside_emode: u8,
    pub utilization_limit_block_borrowing_above_pct: u8,

    #[cfg_attr(feature = "serde", serde(with = "serde_bool_u8"))]
    pub autodeleverage_enabled: u8,
    
    #[cfg_attr(feature = "serde", serde(with = "serde_bool_u8"))]
    pub proposer_authority_locked: u8,
    
    pub borrow_limit_outside_elevation_group: u64,
    pub borrow_limit_against_this_collateral_in_elevation_group: [u64; 32],
    pub deleveraging_bonus_increase_bps_per_day: u64,
    pub debt_maturity_timestamp: u64,
    pub debt_term_seconds: u64,
}