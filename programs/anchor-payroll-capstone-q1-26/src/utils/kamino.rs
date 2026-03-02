use anchor_lang::prelude::*;
use ::solana_program::hash::hash;

pub const KAMINO_PROGRAM_ID: Pubkey = pubkey!("KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD");
//pub const KAMINO_PROGRAM_ID: Pubkey = pubkey!("KLend2g3cPENfacJ1B3121X7A62BwY75q25w1d8nLZk");
pub const USDC_MINT: Pubkey = pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
pub const BOUNTY_AMOUNT: u64 = 100_000;
pub const PLATFORM_TAX: u64 = 50;

pub fn get_sighash(name: &str) -> [u8; 8] {
    let mut sighash = [0u8; 8];

    //let preimage = format!("global:{}", name); 
    //sighash.copy_from_slice(&hash(preimage.as_bytes()).to_bytes()[..8]);

    let image = [b"global:", name.as_bytes()].concat();
    sighash.copy_from_slice(&hash(&image).to_bytes()[..8]);
    sighash
}