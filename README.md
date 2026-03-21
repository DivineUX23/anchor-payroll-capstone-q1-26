# Atlas Payroll

A Solana smart contract that streams employee salaries per second and routes idle employer capital into [Kamino Finance](https://kamino.finance) to earn yield, instead of letting it sit dormant in a commercial bank account.

Built with Rust, Anchor, and tested against a cloned mainnet Kamino state using Surfpool.

> **Technical article:** [Atlas Payroll: Streaming Payroll with Operational Capital Yield on Solana](https://medium.com/@divineigbinoba23/14c5f917e341?source=friends_link&sk=441c9573649b027356b95998172215f4)

---

## Overview

Atlas Payroll is designed for corporate payroll and treasury management. Employer USDC is deposited into the contract and routed directly into Kamino Finance's USDC lending market, receiving kUSDC (Kamino's yield-bearing collateral token) in return. At the same time, employee salaries accrue every second from the moment their account is initialised and can be withdrawn at any time.

To protect against Kamino illiquidity, a **safety vault** always holds at least two days' worth of total payroll outside of Kamino. An off-chain keeper agent monitors on-chain events and calls the rebalance instruction to top up the vault when needed, earning a **$1 USDC bounty** per successful rebalance call.

---

## How It Works

```
Employer USDC → Atlas Contract → Kamino Finance (earning ~13% APY)
                     │
                     ├── Safety Vault (48h payroll buffer)
                     │         ↑
                     │    Keeper rebalances when claims/withdrawals reduce vault
                     │
                     └── Employee withdraws accrued salary anytime
```

### Five participants

| Participant | Role |
|---|---|
| **Operator** | Deposits USDC, initialises and manages employee accounts, can withdraw non-liability capital |
| **Employee** | Salary accrues per second; withdrawable at any time |
| **Kamino Finance** | External lending protocol earning yield on deposited USDC |
| **Safety Vault** | On-chain USDC token account holding a 48h payroll buffer outside Kamino |
| **Keeper** | Off-chain agent monitoring claim/withdraw/offboard events; calls rebalance and earns $1 USDC bounty |

### Operational flow

1. Operator initialises their `ProtocolVault` account on-chain.
2. Operator deposits USDC → contract routes it to Kamino → Kamino mints kUSDC to the protocol.
3. Operator initialises employee accounts with an annual salary. The contract derives the per-second rate and accrual begins immediately.
4. Keeper monitors on-chain events (employee claims, operator withdrawals, staff offboarding). When the safety vault is drawn down, it calls `rebalance`, which redeems kUSDC from Kamino to top up the vault and earns the bounty.
5. Operator can withdraw capital at any time, but never more than the net assets minus total employee liability.
6. Employees call `staff_claim` to withdraw accrued salary. Funds come from the safety vault first; if insufficient, the shortfall is sourced directly from Kamino.
7. Operator offboards an employee via `staff_offboard`, all outstanding pay is sent immediately, accrual stops, and the employee's rate is removed from the global payroll rate.
8. Once a staff account has a zero outstanding balance, the operator closes it with `collect_staff` and reclaims the SOL rent.

---

## Program Architecture

```
programs/
└── anchor-payroll-capstone-q1-26/
    └── src/
        ├── lib.rs
        ├── state/
        │   ├── protocol_vault.rs     // ProtocolVault struct + impl
        │   └── staff_account.rs      // StaffAccount struct + impl
        ├── instructions/
        │   ├── operator_init.rs      // Initialise protocol vault
        │   ├── deposit.rs            // Deposit USDC → Kamino
        │   ├── withdraw.rs           // Operator withdrawal
        │   ├── rebalance.rs          // Keeper rebalance
        │   ├── staff_init.rs         // Initialise employee account
        │   ├── staff_claim.rs        // Employee salary claim
        │   ├── staff_offboard.rs     // Offboard employee + final pay
        │   └── collect_staff.rs      // Close staff account + reclaim rent
        └── helpers/
            ├── kamino.rs             // get_sighash, constants
            └── reserve.rs            // Kamino Reserve struct (zero-copy)
```

---

## Contract Accounts

### `ProtocolVault`

Holds all operator-level state. One vault per operator (PDA seed: `[b"protocol", operator.key()]`).

```rust
pub struct ProtocolVault {
    pub operator: Pubkey,           // Operator public key
    pub safety_amount: u64,         // USDC balance in safety vault
    pub yield_amount: u64,          // kUSDC balance held in Kamino
    pub global_rate: u64,           // Sum of all active employees' per-second rates
    pub liability: u64,             // Total USDC owed to all employees (as of liability_timestamp)
    pub liability_timestamp: u64,   // Last time liability was computed
}
```

### `StaffAccount`

Holds per-employee state. One account per employee (PDA seed: `[b"staff", staff.key()]`).

```rust
pub struct StaffAccount {
    pub active: bool,           // Whether salary is accruing
    pub rate: u64,              // Per-second pay rate (micro-USDC)
    pub total_claimed: u64,     // Cumulative amount withdrawn
    pub time_started: u64,      // Unix timestamp of account initialisation
    pub time_ended: u64,        // Unix timestamp of offboarding (0 if active)
}
```

**Claimable salary formula:**
```
effective_time = time_ended (if inactive) OR now (if active)
claimable     = (effective_time − time_started) × rate − total_claimed
```

---

## Instructions

| Instruction | Signer | Description |
|---|---|---|
| `operator_init` | Operator | Create and initialise `ProtocolVault` |
| `deposit` | Operator | Deposit USDC → route to Kamino → receive kUSDC |
| `withdraw` | Operator | Withdraw capital (subject to liability ceiling) |
| `rebalance` | Keeper | Top up safety vault from Kamino; pay bounty + platform tax |
| `staff_init` | Operator | Initialise employee account with annual salary |
| `staff_claim` | Employee | Withdraw accrued salary |
| `staff_offboard` | Operator | Send final pay + stop accrual + remove from global rate |
| `collect_staff` | Operator | Close staff account + reclaim SOL rent |

---

## Key Implementation Details

### Kamino CPI

The contract calls Kamino via raw CPI using manually constructed 8-byte instruction discriminators:

```rust
pub fn get_sighash(name: &str) -> [u8; 8] {
    let image = [b"global:", name.as_bytes()].concat();
    let mut sighash = [0u8; 8];
    sighash.copy_from_slice(&hash(&image).to_bytes()[..8]);
    sighash
}
```

Two Kamino instructions are used:
- `deposit_reserve_liquidity`: deposit USDC, receive kUSDC
- `redeem_reserve_collateral`: burn kUSDC, receive USDC + accrued yield

### Kamino's u68.60 fixed-point format

Kamino stores liquidity values using `2^60` as the scale factor not the standard `10^18`:

```rust
let wad: u128 = 1u128 << 60;
```

Using the wrong WAD produces completely incorrect exchange rate calculations.

### u128 alignment: split into two u64s

Kamino's IDL defines several fields as `u128` (requires 16-byte alignment). Because Solana accounts start with an 8-byte discriminator, subsequent fields are statistically 8-byte aligned. `bytemuck` cannot guarantee 16-byte alignment across machines, so these fields are stored as `[u64; 2]` and reconstructed at runtime:

```rust
let borrowed_sf = (k_reserve.liquidity.borrowed_amount_sf[0] as u128)
    | ((k_reserve.liquidity.borrowed_amount_sf[1] as u128) << 64);
```

Source: [github.com/solana-foundation/anchor/issues/3114](https://github.com/solana-foundation/anchor/issues/3114)

### Ceiling division for kUSDC redemption

When calculating kUSDC tokens to burn for a target USDC amount, integer truncation would leave the protocol short. Ceiling division prevents under-redemption:

```rust
let ktoken_to_burn = numerator
    .checked_add(total_pool_usdc)
    .and_then(|x| x.checked_sub(1))
    .and_then(|x| x.checked_div(total_pool_usdc))
    .ok_or(ProgramError::ArithmeticOverflow)?
    as u64;
```

### Safety-vault-first ordering

Employee claims and operator withdrawals always source from the safety vault before touching Kamino. This minimises unnecessary CPI calls and ensures employees are paid during any Kamino illiquidity window.

### `MINIMUM_CLAIM` guard

If the accrued salary is smaller than the gas cost to claim it, the transaction reverts. This prevents gas attrition on per-second streaming.

### Keeper deferred-return guards

The rebalance instruction has two silent `Ok(())` returns one before and one after the Kamino redemption to ensure the keeper never loses a transaction fee attempting a rebalance that can't cover its own cost.

---

## Constants

```rust
pub const KAMINO_PROGRAM_ID: Pubkey = pubkey!("KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD");
pub const USDC_MINT:          Pubkey = pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
pub const BOUNTY_AMOUNT:      u64    = 100_000;  // $1.00 USDC (6 decimal places)
pub const PLATFORM_TAX:       u64    = 50;       // 0.5% (50 basis points)
```

---

## Testing

Tests run against a cloned Kamino mainnet state inside [Surfpool](https://github.com/the-turbin3/surfpool) because Kamino's lending markets do not exist on devnet.

### Setup steps performed in `before()`

**1. Discover the USDC reserve on mainnet**

Query mainnet using `getProgramAccounts` with `memcmp` filters on the lending market address (offset 32) and USDC mint (offset 128):

```typescript
const accounts = await mainnetConn.getProgramAccounts(KAMINO_PROGRAM_ID, {
  filters: [
    { memcmp: { offset: 32,  bytes: LENDING_MARKET.toBase58() } },
    { memcmp: { offset: 128, bytes: USDC.toBase58() } },
  ],
});
```

**2. Derive dependent addresses from reserve data**

```typescript
LENDING_MARKET_AUTHORITY = PublicKey.findProgramAddressSync(
  [Buffer.from("lma"), LENDING_MARKET.toBuffer()],
  KAMINO_PROGRAM_ID
)[0];

RESERVE_LIQUIDITY_SUPPLY = new PublicKey(
  reserveAccount.account.data.subarray(160, 192)
);

const COLLATERAL_MINT_OFFSET = 128 + 1232 + 1200;
RESERVE_COLLATERAL_MINT = new PublicKey(
  reserveAccount.account.data.subarray(COLLATERAL_MINT_OFFSET, COLLATERAL_MINT_OFFSET + 32)
);
```

**3. Hijack the USDC mint authority**

The cloned mainnet USDC mint is controlled by Circle. Override the authority via `surfnet_setAccount` to enable test minting:

```typescript
usdcData.writeUInt32LE(1, 0);
operator.publicKey.toBuffer().copy(usdcData, 4);
// push back via surfnet_setAccount, then mintTo(...)
```

**4. Zero out the reserve's stale-slot counter**

Kamino rejects deposits if the reserve hasn't been refreshed in the current slot. Zero the counter at byte offset 16:

```typescript
reserveData.writeBigInt64LE(BigInt(0), 16);
// push back via surfnet_setAccount
```

### Running the tests

```bash
# Build the program
anchor build

# Start Surfpool (separate terminal)
surfpool --start

# Deploy (recommended before first run to avoid WebSocket timing issues)
solana program deploy target/deploy/anchor_payroll_capstone_q1_26.so \
  --program-id target/deploy/anchor_payroll_capstone_q1_26-keypair.json \
  -u localhost \
  --use-rpc

# Run tests
anchor test --skip-local-validator --skip-deploy
```

### Test coverage

| Test | Assertion |
|---|---|
| Operator initialised | `ProtocolVault` created with correct operator key and zeroed fields |
| Staff initialised | `StaffAccount` active, rate > 0, `global_rate` updated on protocol |
| Deposit complete | kUSDC balance increases, `yield_amount` updated |
| Rebalance complete | Safety amount increases, yield amount decreases, keeper and platform paid |
| Withdraw complete | Operator USDC balance increases, safety/yield amounts decrease |
| Staff claim complete | Employee USDC balance increases, `total_claimed` updated, liability decreases |
| Offboarding complete | `active = false`, `time_ended` set, `global_rate` decreases |
| Account closure | Operator SOL balance increases (rent reclaimed) |

---

## Dependencies

```toml
[dependencies]
anchor-spl  = { version = "0.32.1" }
solana-program = "2.3.0"
bytemuck    = { version = "1.20.0", features = ["min_const_generics"] }
```

| Crate | Purpose |
|---|---|
| `anchor-spl` | SPL token account types, `transfer_checked`, `CpiContext` |
| `solana-program` | `hash` for Kamino instruction discriminator construction |
| `bytemuck` | Zero-copy deserialisation of the Kamino `Reserve` account |

---

## References

- [Kamino Finance — klend source](https://github.com/Kamino-Finance/klend/tree/master)
- [Kamino fixed-point format (deepwiki)](https://deepwiki.com/Kamino-Finance/klend)
- [Anchor u128 alignment issue](https://github.com/solana-foundation/anchor/issues/3114)
- [Anchor account types — Box](https://www.anchor-lang.com/docs/references/account-types)

---

## License

MIT
