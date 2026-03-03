use anchor_lang::prelude::*;

pub mod instructions;
pub mod state;
pub mod utils;

pub use instructions::*;
pub use state::*;
pub use utils::*;

declare_id!("2abcDeZg7dsyk4PKG52oauYCuNg8sQHjduYW5WCLBJfz");

#[program]
pub mod anchor_payroll_capstone_q1_26 {
    use super::*;

    pub fn operator_init(ctx: Context<OperatorInit>) -> Result<()> {
        ctx.accounts.init()
    }

    pub fn deposit(ctx: Context<Deposit>, deposit: u64) -> Result<()> {
        ctx.accounts.transfer(deposit)
    }

    pub fn cfo_withdraw(ctx: Context<CFOWithdraw>, amount: u64) -> Result<()> {
        ctx.accounts.withdraw(amount, &ctx.bumps)
    }

    pub fn rebalance(ctx: Context<Rebalance>) -> Result<()> {
        ctx.accounts.rebalance_pay(&ctx.bumps)
    }

    pub fn staff_init(ctx: Context<StaffInit>, annualized_salary: u64) -> Result<()> {
        ctx.accounts.init(annualized_salary)
    }

    pub fn staff_claim(ctx: Context<StaffClaim>) -> Result<()> {
        ctx.accounts.claim(&ctx.bumps)
    }

    pub fn staff_offboard(ctx: Context<StaffOffboard>) -> Result<()> {
        ctx.accounts.claim_and_close(&ctx.bumps)
    }


}

#[derive(Accounts)]
pub struct Initialize {}
