use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct StaffAccount {
    pub active: bool,
    pub rate: u64,
    pub total_claimed: u64,
    pub time_started: u64,
    pub time_ended: u64
}
impl StaffAccount {
    pub fn claimable_salary(&mut self) -> Result<u64> {

        let current_time = if !self.active && self.time_ended != 0 {
            self.time_ended
        } else {
            Clock::get()?.unix_timestamp as u64
        };

        let time_passed = current_time
            .checked_sub(self.time_started)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        let claimable_salary = self.rate
            .checked_mul(time_passed)
            .and_then(|x| x.checked_sub(self.total_claimed))
            .ok_or(ProgramError::ArithmeticOverflow)?;

        Ok(claimable_salary)
    }
}