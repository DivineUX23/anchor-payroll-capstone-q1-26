use anchor_lang::prelude::*;
use ::solana_program::hash::hash;

pub const KAMINO_PROGRAM_ID: Pubkey = pubkey!("KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD");

pub fn get_sighash(name: &str) -> [u8; 8] {
    let mut sighash = [0u8; 8];
    let image = [b"global", name.as_bytes()].concat();
    sighash.copy_from_slice(&hash(&image).to_bytes()[..8]);
    sighash
}