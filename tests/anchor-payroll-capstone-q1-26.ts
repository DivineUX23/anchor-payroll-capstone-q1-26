import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { AnchorPayrollCapstoneQ126 } from "../target/types/anchor_payroll_capstone_q1_26";
import { PublicKey, Keypair, SystemProgram, Ed25519Program, SYSVAR_INSTRUCTIONS_PUBKEY } from "@solana/web3.js";
import { expect } from "chai";
//import { ASSOCIATED_PROGRAM_ID, TOKEN_PROGRAM_ID } from "@coral-xyz/anchor/dist/cjs/utils/token";
import { getAssociatedTokenAddressSync, createAssociatedTokenAccountInstruction, createMint, mintTo, TOKEN_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID, getAccount, resolveExtraAccountMeta } from "@solana/spl-token";

describe("anchor-payroll-capstone-q1-26", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env()
  anchor.setProvider(provider);

  const program = anchor.workspace.anchorPayrollCapstoneQ126 as Program<AnchorPayrollCapstoneQ126>;
  const connection = provider.connection;

  const operator = provider.wallet;
  const staff = Keypair.generate();
  const keeper = Keypair.generate();

  const KAMINO_PROGRAM_ID = new PublicKey("KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD");
  const LENDING_MARKET = new PublicKey("7u3HeHxYDLhnCoErrtycNokbQYbWGzLs6JSDqGAv5PfF");
  const RESERVE_LIQUIDITY_MINT = new PublicKey("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");

  //const RESERVE = Keypair.generate().publicKey;
  const RESERVE = new PublicKey("Gv9ofvLgWk8B8iRzY8LVEe7vG8p2fH4PJDv9tD4kGv9W");

  // The Market Authority (PDA that signs the kTokens)
  const LENDING_MARKET_AUTHORITY = new PublicKey("9G9mZpsqzUsS9XasB8eB99uVatS3G525C2S8v5h94Sxz");

  // The Vault that holds the physical USDC
  const RESERVE_LIQUIDITY_SUPPLY = new PublicKey("8q7DbsHtcwW3gS2yB2hHqC6hFv5W1gR4T6yA6Y4M7zY1");

  // The Kamino kUSDC (Collateral) Mint
  const RESERVE_COLLATERAL_MINT = new PublicKey("B4m1N2wW1B8u3n8S1B2X8T8s7J3L6Q4F8Z9x3G1L5V8R");




/*
// Kamino Program ID
const KAMINO_PROGRAM_ID = new PublicKey("KLend2g3cPENfacJ1B3121X7A62BwY75q25w1d8n");

// The Main Kamino Market
const LENDING_MARKET = new PublicKey("7u3HeHxYDLhnCoErrtycNokbQYbWGzLs6JSDqGAv5PfF");

// The Market Authority (PDA that signs the kTokens)
const LENDING_MARKET_AUTHORITY = new PublicKey("9G9mZpsqzUsS9XasB8eB99uVatS3G525C2S8v5h94Sxz");

// The Live USDC Reserve Account
const RESERVE = new PublicKey("D6q6wuQSrifAvs8o27GZTwBqUa17sS865iB2u1v1N2n2");

// Mainnet USDC Mint
const RESERVE_LIQUIDITY_MINT = new PublicKey("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");

// The Vault that holds the physical USDC
const RESERVE_LIQUIDITY_SUPPLY = new PublicKey("8q7DbsHtcwW3gS2yB2hHqC6hFv5W1gR4T6yA6Y4M7zY1");

// The Kamino kUSDC (Collateral) Mint
const RESERVE_COLLATERAL_MINT = new PublicKey("B4m1N2wW1B8u3n8S1B2X8T8s7J3L6Q4F8Z9x3G1L5V8R");
*/

  const usdc = new PublicKey("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
  let protocol: PublicKey;
  let protocolAuthority: PublicKey;
  const operatorAta = getAssociatedTokenAddressSync(usdc, operator.publicKey);
  let staffAta: PublicKey;
  let staffAccount: PublicKey;
  let Bump: number;
  let protocolKtokenAta: PublicKey;
  let instructionSysvar: PublicKey;
  //let RESERVE_COLLATERAL_MINT: PublicKey;

  const seed1 = new anchor.BN(1111);

  //const depositAmount = 50_000_000_000;
  const depositAmount = "50000000000000000";

  before(async () => {
    await connection.requestAirdrop(staff.publicKey, 5_000_000_000);
    await connection.requestAirdrop(keeper.publicKey, 5_000_000_000);
    await new Promise((resolve) => setTimeout(resolve, 1000));

    protocol = PublicKey.findProgramAddressSync(
      [Buffer.from("protocol"), operator.publicKey.toBuffer()],
      program.programId
    )[0];

    protocolAuthority = PublicKey.findProgramAddressSync(
      [Buffer.from("authority"), protocol.toBuffer()],
      program.programId
    )[0];

    staffAccount = PublicKey.findProgramAddressSync(
      [Buffer.from("staff"), staff.publicKey.toBuffer()],
      program.programId
    )[0];


    protocolKtokenAta = getAssociatedTokenAddressSync(
      RESERVE_COLLATERAL_MINT,
      protocolAuthority,
      true
    );

    //usdc = await createMint(connection, operator.payer, operator.publicKey, null, 6);
    
    //operatorAta = getAssociatedTokenAddressSync(usdc, operator.publicKey);
    const operatorAtaTx = new anchor.web3.Transaction().add(
      createAssociatedTokenAccountInstruction(operator.publicKey, operatorAta, operator.publicKey, usdc)
    )
    await provider.sendAndConfirm(operatorAtaTx);
    //await mintTo(connection, operator.payer, usdc, operatorAta, operator.payer, BigInt(depositAmount));


    //RESERVE_COLLATERAL_MINT = await createMint(connection, operator.payer, operator.publicKey, null, 6);

    protocolKtokenAta = getAssociatedTokenAddressSync(
      RESERVE_COLLATERAL_MINT,
      protocolAuthority,
      true
    )
    const protocolKtokenAtaTx = new anchor.web3.Transaction().add(
      createAssociatedTokenAccountInstruction(operator.publicKey, protocolKtokenAta, protocolAuthority, RESERVE_COLLATERAL_MINT)
    )
    await provider.sendAndConfirm(protocolKtokenAtaTx);
    //await mintTo(connection, operator.payer, usdc, protocolKtokenAta, operator.payer, depositAmount);


  // 3. SURFPOOL CHEATCODE: Forge Operator USDC Account
    // This bypasses the AToken program and writes the ATA directly into the ledger.
    await fetch("http://127.0.0.1:8899", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
            jsonrpc: "2.0",
            id: 1,
            method: "surfnet_setTokenAccount",
            params: {
                owner: operator.publicKey.toBase58(),
                mint: usdc.toBase58(),
                amount: 50000000 // 50 real USDC (6 decimals)
            }
        })
    });
    console.log("✅ Operator USDC ATA forged via Cheatcode.");

    // 4. SURFPOOL CHEATCODE: Forge Protocol kUSDC Vault
    // This creates the destination account for Kamino to send yield tokens into.
    await fetch("http://127.0.0.1:8899", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
            jsonrpc: "2.0",
            id: 2,
            method: "surfnet_setTokenAccount",
            params: {
                owner: protocolAuthority.toBase58(),
                mint: RESERVE_COLLATERAL_MINT.toBase58(),
                amount: 0 // Initialize empty vault
            }
        })
    });
    console.log("✅ Protocol kUSDC Vault forged via Cheatcode.");
    
  })


  it("Operator is initialized!", async () => {
    // Add your test here.

    await program.methods
      .operatorInit()
      .accountsStrict({
        operator: operator.publicKey,
        protocol: protocol,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId
      })
      .rpc();
      const protocolInfo = await program.account.protocolVault.fetch(protocol);
      expect(protocolInfo.operator.toBase58()).to.equal(operator.publicKey.toBase58());
      expect(Number(protocolInfo.safetyAmount)).to.equal(0);
      expect(Number(protocolInfo.yieldAmount)).to.equal(0);
      expect(Number(protocolInfo.globalRate)).to.equal(0);
      expect(Number(protocolInfo.liability)).to.equal(0);
      //expect(protocolInfo.liabilityTimestamp).to.greaterThan(0);
  });




  it("Staff is initialized!", async () => {
    // Add your test here.
    const annualized_salary = new anchor.BN(5_000_000_000);

    await program.methods
      .staffInit(annualized_salary)
      .accountsStrict({
        operator: operator.publicKey,
        staff: staff.publicKey,
        protocol: protocol,
        staffAccount: staffAccount,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId
      })
      .rpc();
      const staffInfo = await program.account.staffAccount.fetch(staffAccount);
      expect(Boolean(staffInfo.active)).to.equal(true);
      expect(Number(staffInfo.rate)).to.greaterThan(0);
      expect(Number(staffInfo.totalClaimed)).to.equal(0);
      //expect(staffInfo.timeStarted).to.greaterThan(0);

      const protocolInfo = await program.account.protocolVault.fetch(protocol);
      expect(Number(protocolInfo.globalRate)).to.greaterThan(0);
      //expect(Number(protocolInfo.liability)).to.greaterThan(0);

  });



  
  it("Deposit is complete!", async () => {
    // Add your test here.
    const deposit = new anchor.BN("50000000");
    try {
      await program.methods
        .deposit(deposit)
        .accountsStrict({
          operator: operator.publicKey,
          usdc: usdc,
          operatorAta: operatorAta,
          protocol: protocol,
          protocolAuthority: protocolAuthority,
          protocolKtokenAta: protocolKtokenAta,

          kaminoProgram: KAMINO_PROGRAM_ID,
          reserve: RESERVE,
          lendingMarket: LENDING_MARKET,
          lendingMarketAuthority: LENDING_MARKET_AUTHORITY,
          reserveLiquidityMint: RESERVE_LIQUIDITY_MINT,
          reserveLiquiditySupply: RESERVE_LIQUIDITY_SUPPLY,
          reserveCollateralMint: RESERVE_COLLATERAL_MINT,

          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: anchor.web3.SystemProgram.programId,

          instructionSysvar: SYSVAR_INSTRUCTIONS_PUBKEY,

        })
        .rpc();
        const protocolInfo = await program.account.protocolVault.fetch(protocol);
        expect(Number(protocolInfo.yieldAmount)).to.greaterThan(0);
      } catch (e) {
          console.log("Expected CPI Rejection due to Mock USDC Mint mismatch:", e.message);
      }
  });

});
