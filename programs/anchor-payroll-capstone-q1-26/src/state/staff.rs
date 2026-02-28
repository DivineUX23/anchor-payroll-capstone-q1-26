use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct StaffAccount {
    pub active: bool,
    pub rate: u8,
    pub total_claimed: u64,
    pub time_started: u64
}