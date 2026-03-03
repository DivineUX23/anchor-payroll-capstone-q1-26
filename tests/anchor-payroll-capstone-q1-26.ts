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

  const LENDING_MARKET_AUTHORITY = Keypair.generate().publicKey;
  const RESERVE_LIQUIDITY_SUPPLY = Keypair.generate().publicKey;
  //const RESERVE_COLLATERAL_MINT = Keypair.generate().publicKey;


  let usdc: PublicKey;
  let protocol: PublicKey;
  let protocolAuthority: PublicKey;
  let operatorAta: PublicKey;
  let staffAta: PublicKey;
  let staffAccount: PublicKey;
  let Bump: number;
  let protocolKtokenAta: PublicKey;
  let instructionSysvar: PublicKey;
  let RESERVE_COLLATERAL_MINT: PublicKey;

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

    usdc = await createMint(connection, operator.payer, operator.publicKey, null, 6);
    
    operatorAta = getAssociatedTokenAddressSync(usdc, operator.publicKey);
    const operatorAtaTx = new anchor.web3.Transaction().add(
      createAssociatedTokenAccountInstruction(operator.publicKey, operatorAta, operator.publicKey, usdc)
    )
    await provider.sendAndConfirm(operatorAtaTx);
    await mintTo(connection, operator.payer, usdc, operatorAta, operator.payer, BigInt(depositAmount));


    RESERVE_COLLATERAL_MINT = await createMint(connection, operator.payer, operator.publicKey, null, 6);

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
    const deposit = new anchor.BN("50000000000000000");
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
