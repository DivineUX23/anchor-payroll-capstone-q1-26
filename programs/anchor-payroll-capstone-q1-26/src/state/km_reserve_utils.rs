use anchor_lang::prelude::*;
use crate::state::{PriceHeuristic, ScopeConfiguration, SwitchboardConfiguration, PythConfiguration};

pub const TOKEN_INFO_SIZE: usize = 384;


//static_assertions::const_assert_eq!(TOKEN_INFO_SIZE, std::mem::size_of::<TokenInfo>());
//static_assertions::const_assert_eq!(0, std::mem::size_of::<TokenInfo>() % 8);
#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[zero_copy]
#[repr(C)]
pub struct TokenInfo {
    #[cfg_attr(feature = "serde", serde(with = "serde_name"))]
    pub name: [u8; 32],
    pub heuristic: PriceHeuristic,
    pub max_twap_divergence_bps: u64,
    pub max_age_price_seconds: u64,
    pub max_age_twap_seconds: u64,
    #[cfg_attr(feature = "serde", serde(default))]
    pub scope_configuration: ScopeConfiguration,
    #[cfg_attr(feature = "serde", serde(default))]
    pub switchboard_configuration: SwitchboardConfiguration,
    #[cfg_attr(feature = "serde", serde(default))]
    pub pyth_configuration: PythConfiguration,
    pub block_price_usage: u8,
    #[cfg_attr(feature = "serde", serde(skip_serializing, default))]
    pub reserved: [u8; 7],
    #[cfg_attr(feature = "serde", serde(skip_serializing, default))]
    pub padding: [u64; 19],
}

#[derive(AnchorSerialize, AnchorDeserialize, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[zero_copy]
#[repr(C)]
pub struct CurvePoint {
    pub utilization_rate_bps: u32,
    pub borrow_rate_bps: u32,
}

#[derive(AnchorSerialize, AnchorDeserialize, Default, Debug, PartialEq, Eq)]
#[zero_copy]
#[repr(C)]
pub struct BorrowRateCurve {
    pub points: [CurvePoint; 11],
}

#[derive(AnchorSerialize, AnchorDeserialize, Default, PartialEq, Eq)]//, Derivative)]
//#[derivative(Debug)]
#[zero_copy]
#[repr(C)]
pub struct ReserveFees {
    pub origination_fee_sf: u64,
    pub flash_loan_fee_sf: u64,

    //#[derivative(Debug = "ignore")]
    pub padding: [u8; 8],
}


#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Eq, Default, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[zero_copy]
#[repr(C)]
pub struct WithdrawalCaps {
    pub config_capacity: i64,
    #[cfg_attr(
        all(feature = "serde", not(feature = "serialize_caps_interval_values")),
        serde(skip)
    )]
    pub current_total: i64,
    #[cfg_attr(
        all(feature = "serde", not(feature = "serialize_caps_interval_values")),
        serde(skip)
    )]
    pub last_interval_start_timestamp: u64,
    pub config_interval_length_seconds: u64,
}