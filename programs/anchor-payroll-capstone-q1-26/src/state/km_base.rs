use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Debug)]
#[zero_copy]
#[repr(C)]
pub struct LastUpdate {
    pub slot: u64,
    pub stale: u8,
    pub price_status: u8,
    pub placeholder: [u8; 6],
}
impl PartialEq for LastUpdate {
    fn eq(&self, other: &Self) -> bool {
        self.slot == other.slot
    }
}


#[derive(Default, Debug, PartialEq, Eq)]
#[zero_copy]
#[repr(C)]
pub struct BigFractionBytes {
    pub value: [u64; 4],
    pub padding: [u64; 2],
}

#[derive(AnchorSerialize, AnchorDeserialize, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[zero_copy]
#[repr(C)]
pub struct PriceHeuristic {
    pub lower: u64,
    pub upper: u64,
    pub exp: u64,
}


#[derive(AnchorSerialize, AnchorDeserialize, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[zero_copy]
#[repr(transparent)]
pub struct PythConfiguration {
    #[cfg_attr(feature = "serde", serde(with = "serde_string", default))]
    pub price: Pubkey,
}


#[derive(AnchorSerialize, AnchorDeserialize, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[zero_copy]
#[repr(C)]
pub struct ScopeConfiguration {
    pub price_feed: Pubkey,
    pub price_chain: [u16; 4],
    pub twap_chain: [u16; 4],
}

#[derive(AnchorSerialize, AnchorDeserialize, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[zero_copy]
#[repr(C)]
pub struct SwitchboardConfiguration {

    #[cfg_attr(feature = "serde", serde(with = "serde_string", default))]
    pub price_aggregator: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "serde_string", default))]
    pub twap_aggregator: Pubkey,
}