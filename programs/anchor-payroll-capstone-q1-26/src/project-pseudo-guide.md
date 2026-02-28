pub struct ProtocolVault {

    //pub max_amount: u64,
    pub safety_current_amount: u64,
    pub yield_total_amount: u64,

    pub global_active_rate: u64,
    pub accumulated_liability: u64,
    pub last_liability_update_timestamp: u64
}

impl ProtocolVault {
    pub fn update_global_liabiltiy() {
        let current_time = time_now_sec();
        let time_delta = current_time - self.last_liability_update_timestamp;

        self.accumulated_liability += (self.global_active_rate * time_delta);
        self.last_liability_update_timestamp = self.current_time;

    }

    pub fn calculate_liquid_capital(
        &self, 
        kamino_reserve_info: &AccountInfo
    ) {

        let reserve_data = kamino_reserve_info.data.borrow();
        let kamino_reserve = Reserve::try_deserialize(&mut &reserve_data[..])
            .expect("Failed to deserialize Kamino Reserve");

        let WAD: u128 = 1_000_000_000_000_000_000;

        let available_liquidity = kamino_reserve.liquidity.available_amount as u128;
        let borrowed_liquidity = kamino_reserve.liquidity.borrowed_amount_sf / WAD; 
        let protocol_fees = kamino_reserve.liquidity.accumulated_protocol_fees_sf / WAD;

        let total_pool_liquidity_lamports = available_liquidity 
            + borrowed_liquidity 
            - protocol_fees;

        let total_ktoken_supply = kamino_reserve.collateral.mint_total_supply as u128;
        if total_ktoken_supply == 0 {
            return Ok(self.safety_current_amount); // Pool is empty, fallback to safety buffer
        }

        let my_ktoken_balance = self.yield_total_amount as u128; 

        let my_true_usdc_value = (my_ktoken_balance * total_pool_liquidity_lamports) / total_ktoken_supply;


        //let total_asset = self.yield_total_amount + self.safety_current_amount;
        let total_assets = self.safety_current_amount + (my_true_usdc_value as u64);

        let current_liability = self.accumulated_liability + (self.global_active_rate * (time_now_sec() - self.last_liability_update_timestamp));

        if total_asset > current_liability {
            Ok(total_asset -= current_liability)
        } else {
            Ok(0)
        }
    }

    pub fn update_protocol_vault() {
        let daily_burn_rate = self.global_active_rate * 3600 * 24;
        let two_days = 3600 * 48;
        
        let protocol_vault_target = (daily_burn_rate * two_days * 12) / 10;
        // protocol_vault_target = daily_burn_rate * two_days * 1.2;

        if self.safety_current_amount < protocol_vault_target {
            send_to_safety = protocol_vault_target - self.safety_current_amount;
            Ok(send_to_safety)

        } else {
            Ok(0)
        }
    }

}


/*pub struct YieldVault {

    pub total_yield: u64,
    pub yield_pnl: u64,
    pub amount_added: u64,
    
}*/


pub struct StaffAccount {
    //pub salary: u64,
    pub active: bool,
    // pub claimable_salary: u64, // this may go
    pub rate_per_sec: u8,
    //pub last_claim_timestamp: u64,
    pub total_claimed: u64, // this may go
    pub time_started: u64
}



// NOT ALL WITHDRAW FROM YIELD, BUFFER IS PURELY FALL BACK



pub struct CFOInit {
    pub usd_pda,
    pub cfo_pda,

    [account(authority=cfo_pda, payer=cfo_pda)]
    pub protocol_vault <'info, ProtocolVault>,

    //[account(authority=cfo_pda)] 
   // pub yield_vault <'info, YieldVault>
}

impl CFOInit {
    self.protocol_vault.safety_current_amount = 0;
    self.protocol_vault.yield_total_amount = 0;
    self.protocol_vault.global_active_rate = 0;
    self.protocol_vault.accumulated_liability = 0;
    self.protocol_vault.last_liability_update_timestamp = 0;


    //yield_vault.total_yield = 0;
    //yield_vault.yield_pnl = 0;
    //yield_vault.amount_added = 0;

}




[derive(Accounts)]
pub struct CFODeposit {
    pub usd_pda,
    pub cfo_pda,

    [account(usd_pda, cfo_pda)]
    pub cfo_ata Account<'info, Mint>,

    [account(authority=cfo_pda, payer=cfo_pda)]
    pub protocol_vault <'info, ProtocolVault>,

}


//UNPACK DATA FROM FRONTEND IN REAL IMPL, NOT SENDING RAW AMOUNT AND PERCENTAGE!
impl CFODeposit {
    pub fn transfer (&mut self, deposit_amount: u64) -> Result(()) {

        if deposit_amount <= 0 {
            return Err(ErrorCode::ZeroFunds);
        }

        self.protocol_vault.update_global_liability();

        let ktoken_balance_before = self.protocol_ktokn_ata.amount;

        let cpi_program = self.kamino_program.to_account_info();

        let cpi_accounts = DepositReserveLiquidity {
            reserve: self.reserve.to_account_info(),
            lending_market: self.lending_market.to_account_info(),
            lending_market_authority: self.lending_market_authority.to_account_info(),
            reserve_liquidity_supply: self.reserve_liquidity_supply.to_account_info(),
            reserve_collateral_mint: self.reserve_collateral_mint.to_account_info(),
            
            // Source: CFO's raw USDC (Pulled directly via CFO's signature on the transaction)
            user_source_liquidity: self.cfo_usdc_ata.to_account_info(), 
            
            // Destination: Protocol's kUSDC vault (The protocol captures the yield token)
            user_destination_collateral: self.protocol_ktokn_ata.to_account_info(),
            
            // Authority: The CFO must sign to authorize the pull from their wallet
            user_transfer_authority: self.cfo.to_account_info(),
            token_program: self.token_program.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

        // 3. Execution: Deploy Capital directly to Kamino
        kamino_lending_interface::cpi::deposit_reserve_liquidity(
            cpi_ctx, 
            deposit_amount
        )?;


        // 3. Force an SVM State Refresh
        // This is mathematically mandatory. It commands the runtime to fetch the 
        // new data bytes Kamino just wrote to your kUSDC TokenAccount.
        self.protocol_ktokn_ata.reload()?;

        // 4. Snapshot the post-execution state
        let ktoken_balance_after = self.protocol_ktokn_ata.amount;

        // 5. Calculate the absolute exact yield-bearing asset delta
        let exact_ktokens_minted = ktoken_balance_after
            .checked_sub(ktoken_balance_before)
            .expect("Mathematical underflow during kToken delta calculation");

        // 6. Update the global ledger using the receipt token metric
        // We add the exact kTokens to the vault's tracking state, NOT the raw USDC deposit amount.
        self.protocol_vault.yield_total_amount += exact_ktokens_minted;

        Ok(())

    }

}





// Called by BOT or ConJob
pub struct Rebalance {
    pub keeper: Signer<'info>,

    [account(authority=cfo_pda, payer=cfo_pda)]
    pub protocol_vault <'info, ProtocolVault>,

    pub vault_liquidity_account <'info, TokenAccount>,

    pub keeper_token_account <'info, TokenAccount>,

    //[account(authority=cfo_pda)] 
    //pub yield_vault <'info, YieldVault>
}

impl Rebalance {
    pub fn rebalance {

        let bounty_amount: u64 = 100_000;

        // Platform Tax: 0.5% (50 bps) of the total capital being moved
        let platform_tax: u64 = deposit_amount
            .checked_mul(50).unwrap()
            .checked_div(10000).unwrap();

        let total_deduction = bounty_amount.checked_add(platform_tax).unwrap();

        if self.protocol_vault.yield_total_amount < total_deduction {
            msg!("Warning: Company vault cannot afford keeper bounty.");
            return Ok(());
        }

        let required_extraction = self.protocol_vault.update_protocol_vault()

        if required_extraction <= 0 {
            msg!("Warning: Protocol is already balanced.");
            return Ok(()); 
        }

        let available_kamino_liquidity = read_kamino_reserve_liquidity(&self.accounts.kamino_reserve)?;
        
        if available_kamino_liquidity < required_extraction {
            msg!("Warning: Lending pool illiquid. Extraction deferred.");
            return Ok(()); 
        }      

        // Execute CPI to Kamino to withdraw `required_extraction`
        exact_ktokens_minted = self.ExtractAndClaimJIT.execute_kamino_withdrawal(self, required_extraction)?;


        self.protocol_vault.yield_total_amount -= exact_ktokens_minted;

        // Send bounty
        let keeper_cpi = CpiContext::new_with_signer(
            self.token_program.to_account_info(), 
            Transfer {
                from: self.vault_liquidity_account.to_account_info(),
                to: self.keeper_token_account.to_account_info(),
                authority: self.protocol_vault.to_account_info(),
            }, 
            protocol_signer_seeds
        );
        token::transfer(keeper_cpi, bounty_amount)?;

        // Extract Protocol Rent (Platform Tax)
        let tax_cpi = CpiContext::new_with_signer(
            self.token_program.to_account_info(), 
            Transfer {
                from: self.vault_liquidity_account.to_account_info(),
                to: self.platform_treasury_account.to_account_info(),
                authority: self.protocol_vault.to_account_info(),
            }, 
            protocol_signer_seeds
        );
        token::transfer(tax_cpi, platform_tax)?;


        self.protocol_vault.safety_current_amount += required_extraction;
        //self.protocol_vault.yield_total_amount -= (required_extraction + total_deduction);

        Ok(())
    }

    pub fn execute_kamino_withdrawal(&mut self, amount: u64) {

        //CPI to Kamino;
    }
}




// WITHDRAW
[derive(Accounts)]
pub struct CFOWithdraw  {
    pub usd_pda,
    pub cfo_pda,
    pub staff_pda

    [account(usd_pda, cfo_pda)]
    pub cfo_ata Account<'info, Mint>,

    [account(authority=cfo_pda, payer=cfo_pda)]
    pub protocol_vault <'info, ProtocolVault>,

    //[account(authority=cfo_pda)] 
    //pub yield_vault <'info, YieldVault>

    [account(authority=staff_pda)] 
    pub staff_account<'info, StaffAccount>

}


impl CFOWithdraw  {
    pub fn transfer (&mut self, requested_amount: u64) {

        let liquid_capital = self.protocol_vault.calculate_liquid_capital();

        if requested_amount > liquid_capital {
            panic!("Withdrawal exceeds non-liable treasury assets.");
        }
            
        if (requested_amount < self.protocol_vault.yield_total_amount) {
            self.send_to_cfo_from_yield(requested_amount);

        } else {
            safety_drain = requested_amount - self.protocol_vault.yield_total_amount;

            self.send_to_cfo(safety_drain);
            self.send_to_cfo_from_yield(self.protocol_vault.yield_total_amount);
        }

    }

    pub fn send_to_cfo(requested_amount: u64){
        Ok(())
    }

    pub fn send_to_cfo_from_yield(requested_amount: u64){
        Ok(())
    }

}




// INIT STAFF
[derive(Accounts)]
pub struct StaffInit {
    pub usd_pda,
    pub staff_pda

    [account(usd_pda, staff_pda)]
    pub staff_ata Account<'info, Mint>,

    [account(authority=cfo_pda, payer=cfo_pda)]
    pub protocol_vault <'info, ProtocolVault>,

    [account(authority=staff_pda)] 
    pub staff_account<'info, StaffAccount>
}

impl StaffInit {

    pub fn intialize(&mut self, annualized_salary) {

        // One time - Incase the employee has no gas for future withdrawal
        //if (self.staff_pda.gas < 0.002) {
            //to_transfer = 0.002 - self.staff_pda.gas
            //transfer_to_staff_pda(to_transfer)
        //}

        self.protocol_vault.update_global_liabiltiy();

        let rate = salary / 31_557_600 // 365.25 days might go back to months ---> let rate = salary / (30*24*3600)

        self.staff_account.active = true,
        self.staff_account.rate_per_sec = rate,
        self.staff_account.time_started = time_now_sec(),
        self.staff_account.total_claimed = 0,

        self.protocol_vault.global_active_rate += rate

    }

    //pub fn transfer_to_staff_pda(to_transfer: u64){
        //Ok(())
    //}

}






// Claim Staff

pub struct StaffClaim {
    pub usd_pda,
    pub cfo_pda,
    pub staff_pda

    [account(usd_pda, cfo_pda)]
    pub cfo_ata Account<'info, Mint>,

    [account(authority=cfo_pda, payer=cfo_pda)]
    pub protocol_vault <'info, ProtocolVault>,

    //[account(authority=cfo_pda)] 
    //pub yield_vault <'info, YieldVault>

    [account(authority=staff_pda)] 
    pub staff_account<'info, StaffAccount>
}

impl StaffClaim {
    pub fn claim(&mut self){

        self.protocol_vault.update_global_liabiltiy();

        let time_passed = time_now_sec() - self.staff_account.time_started;
        let claimable_salary: u64 = (rate * time_passed) - self.staff_account.total_claimed;
        if claimable_salary == 0 {
            return Err(ErrorCode::ZeroClaim);
        }


        if self.protocol_vault.safety_current_amount >= claimable_salary {
            self.send_to_staff(claimable_salary);
            self.protocol_vault.safety_current_amount -= claimable_salary;

        } else {
            // Error - "insufficient call admin!"
            return Err(ErrorCode::InsufficientProtocolVault);
        }

        self.staff_account.total_claimed += claimable_salary;

        self.protocol_vault.accumulated_liability -= claimable_salary;
    }

    pub fn self.send_to_staff(requested_amount: u64){
        Ok(())
    }
}




// Remove Employee

pub struct StaffRemove {
    pub usd_pda,
    pub cfo_pda,
    pub staff_pda

    [account(usd_pda, cfo_pda)]
    pub cfo_ata Account<'info, Mint>,

    [account(authority=cfo_pda, payer=cfo_pda)]
    pub protocol_vault <'info, ProtocolVault>,

    [account(authority=staff_pda)] 
    pub staff_account<'info, StaffAccount>
}

impl StaffRemove {
    pub fn remove_staff() {
        self.claim();

        self.protocol_vault.update_global_liabiltiy();
        self.protocol_vault.global_active_rate -= self.staff_account.rate_per_sec;

        self.staff_account.active = false,
        self.staff_account.rate_per_sec = 0,

        self.close_staff_account();

    }

    pub fn close_staff_account(
        Ok(())
    )
}