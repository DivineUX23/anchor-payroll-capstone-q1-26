import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { AnchorPayrollCapstoneQ126 } from "../target/types/anchor_payroll_capstone_q1_26";
import { PublicKey, Keypair, SYSVAR_INSTRUCTIONS_PUBKEY } from "@solana/web3.js";
import { expect } from "chai";
import { getAssociatedTokenAddressSync, 
  createAssociatedTokenAccountIdempotentInstruction, 
  mintTo, TOKEN_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID, 
  getAccount } from "@solana/spl-token";

describe("anchor-payroll-capstone-q1-26", () => {

  const provider = anchor.AnchorProvider.env()
  anchor.setProvider(provider);

  const program = anchor.workspace.anchorPayrollCapstoneQ126 as Program<AnchorPayrollCapstoneQ126>;
  const connection = provider.connection;

  const operator = provider.wallet;
  const staff = Keypair.generate();
  const keeper = Keypair.generate();
  const platform = Keypair.generate();

  const KAMINO_PROGRAM_ID = new PublicKey("KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD");
  const LENDING_MARKET = new PublicKey("7u3HeHxYDLhnCoErrtycNokbQYbWGzLs6JSDqGAv5PfF");
  const USDC = new PublicKey("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");


  let RESERVE: PublicKey;
  let LENDING_MARKET_AUTHORITY: PublicKey;
  let RESERVE_LIQUIDITY_SUPPLY: PublicKey;
  let RESERVE_COLLATERAL_MINT: PublicKey;

  let protocol: PublicKey;
  let protocolAuthority: PublicKey;
  let staffAccount: PublicKey;

  let staffAta: PublicKey;
  let keeperAta: PublicKey;
  let protocolAta: PublicKey;
  let platformAta: PublicKey;
  let operatorAta: PublicKey;
  let protocolKtokenAta: PublicKey;


  before(async () => {
    console.log("Fetching Kamino Reserve via raw RPC...");

    const mainnetCon = new anchor.web3.Connection("https://api.mainnet-beta.solana.com");

    const LENDING_MARKET_OFFSET = 32;
    const USDC_OFFSET = 128;
    const LIQ_SUPPLY_VAULT_OFFSET = 160;

    const accounts = await mainnetCon.getProgramAccounts(KAMINO_PROGRAM_ID, {
      filters: [
        { memcmp: { offset: LENDING_MARKET_OFFSET, bytes: LENDING_MARKET.toBase58() }},
        { memcmp: { offset: USDC_OFFSET, bytes: USDC.toBase58() }},
      ],
    });

    if (accounts.length === 0) throw new Error("USDC Reserve not found");
    const reserveAccount = accounts[0];
    RESERVE = reserveAccount.pubkey;

    LENDING_MARKET_AUTHORITY = PublicKey.findProgramAddressSync(
      [Buffer.from("lma"), LENDING_MARKET.toBuffer()],
      KAMINO_PROGRAM_ID
    )[0];
    RESERVE_LIQUIDITY_SUPPLY = new PublicKey(reserveAccount.account.data.subarray(LIQ_SUPPLY_VAULT_OFFSET, LIQ_SUPPLY_VAULT_OFFSET + 32));

    const COLLATERAL_MINT_OFFSET = 128 + 1232 + 1200;
    RESERVE_COLLATERAL_MINT = new PublicKey(reserveAccount.account.data.subarray(COLLATERAL_MINT_OFFSET, COLLATERAL_MINT_OFFSET + 32));


    await connection.getMultipleAccountsInfo([
      USDC,
      RESERVE_COLLATERAL_MINT,
      RESERVE,
      KAMINO_PROGRAM_ID,
      LENDING_MARKET,
      LENDING_MARKET_AUTHORITY,
      RESERVE_LIQUIDITY_SUPPLY
    ]);

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

    protocolAta = getAssociatedTokenAddressSync(USDC, protocolAuthority, true);
    platformAta = getAssociatedTokenAddressSync(USDC, platform.publicKey);
    operatorAta = getAssociatedTokenAddressSync(USDC, operator.publicKey);
    keeperAta = getAssociatedTokenAddressSync(USDC, keeper.publicKey);
    staffAta = getAssociatedTokenAddressSync(USDC, staff.publicKey);

    
    const initAtaTx = new anchor.web3.Transaction().add(
      createAssociatedTokenAccountIdempotentInstruction(operator.publicKey, protocolKtokenAta, protocolAuthority, RESERVE_COLLATERAL_MINT),
      createAssociatedTokenAccountIdempotentInstruction(operator.publicKey, operatorAta, operator.publicKey, USDC),
      createAssociatedTokenAccountIdempotentInstruction(operator.publicKey, protocolAta, protocolAuthority, USDC),
      createAssociatedTokenAccountIdempotentInstruction(operator.publicKey, platformAta, platform.publicKey, USDC),
      createAssociatedTokenAccountIdempotentInstruction(operator.publicKey, keeperAta, keeper.publicKey, USDC),
      createAssociatedTokenAccountIdempotentInstruction(operator.publicKey, staffAta, staff.publicKey, USDC),
    );
    await provider.sendAndConfirm(initAtaTx);


    const usdcInfo = await connection.getAccountInfo(USDC);
    let usdcData = usdcInfo.data;

    usdcData.writeUInt32LE(1, 0);
    operator.publicKey.toBuffer().copy(usdcData, 4);

    const hijackResp = await fetch("http://127.0.0.1:8899", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        jsonrpc: "2.0",
        id: 1,
        method: "surfnet_setAccount",
        params: [
          USDC.toBase58(),
          { 
            lamports: usdcInfo.lamports, 
            data: usdcData.toString("hex"), 
            owner: usdcInfo.owner.toBase58(), 
            executable: false, 
            rentEpoch: 0 
          }
        ]
      })
    });
    await hijackResp.json();

    await new Promise((resolve) => setTimeout(resolve, 2000));

    await mintTo(connection, operator.payer, USDC, operatorAta, operator.payer, 5_000_000_000_000);



    const reserveInfo = await connection.getAccountInfo(RESERVE);
    if (!reserveInfo) throw new Error("Cloned reserve not foound");

    let reserveData = reserveInfo.data;

    reserveData.writeBigInt64LE(BigInt(0), 16);

    const reserveHijackResp = await fetch("http://127.0.0.1:8899", {
      method: "POST",
      headers: { "Content-Type": "application/json"},
      body: JSON.stringify({
        jsonrpc: "2.0",
        id: 2,
        method: "surfnet_setAccount",
        params: [
          RESERVE.toBase58(),
          {
            lamports: reserveInfo.lamports,
            data: reserveData.toString("hex"),
            owner: reserveInfo.owner.toBase58(),
            executable: false,
            rentEpoch: 0
          }
        ]
      })
    })
    await reserveHijackResp.json();

  });




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
    const annualized_salary = new anchor.BN(50_000_000_000_000);

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
    const deposit = new anchor.BN(5_000_000_000_000);
    await program.methods
      .deposit(deposit)
      .accountsStrict({
        operator: operator.publicKey,
        usdc: USDC,
        operatorAta: operatorAta,
        protocol: protocol,
        protocolAuthority: protocolAuthority,
        protocolKtokenAta: protocolKtokenAta,

        kaminoProgram: KAMINO_PROGRAM_ID,
        reserve: RESERVE,
        lendingMarket: LENDING_MARKET,
        lendingMarketAuthority: LENDING_MARKET_AUTHORITY,
        reserveLiquidityMint: USDC,
        reserveLiquiditySupply: RESERVE_LIQUIDITY_SUPPLY,
        reserveCollateralMint: RESERVE_COLLATERAL_MINT,

        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        instructionSysvar: SYSVAR_INSTRUCTIONS_PUBKEY,
      })
      .rpc();
    const protocolInfo = await program.account.protocolVault.fetch(protocol);
    expect(Number(protocolInfo.yieldAmount)).to.greaterThan(0);

  });





  it("Rebalance is complete!", async () => {
    // Add your test here.
    await program.methods
      .rebalance()
      .accountsStrict({
        keeper: keeper.publicKey,
        operator: operator.publicKey,
        platform: platform.publicKey,
        usdc: USDC,
        keeperAta: keeperAta,
        protocol: protocol,
        protocolAuthority: protocolAuthority,

        protocolAta: protocolAta,
        protocolKtokenAta: protocolKtokenAta,
        platformAta: platformAta,

        kaminoProgram: KAMINO_PROGRAM_ID,
        reserve: RESERVE,
        lendingMarket: LENDING_MARKET,
        lendingMarketAuthority: LENDING_MARKET_AUTHORITY,
        reserveLiquidityMint: USDC,
        reserveLiquiditySupply: RESERVE_LIQUIDITY_SUPPLY,
        reserveCollateralMint: RESERVE_COLLATERAL_MINT,

        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        instructionSysvar: SYSVAR_INSTRUCTIONS_PUBKEY,
      })
      .signers([keeper])
      .rpc();
    const protocolInfo = await program.account.protocolVault.fetch(protocol);
    expect(Number(protocolInfo.safetyAmount)).to.greaterThan(0);

    const keeperInfo = await getAccount(provider.connection, keeperAta);
    expect(Number(keeperInfo.amount)).to.greaterThan(0);

    const platformInfo = await getAccount(provider.connection, platformAta);
    expect(Number(platformInfo.amount)).to.greaterThan(0);

  });




  
  it("Withdraw is complete!", async () => {

    const deposit = new anchor.BN(5_000_000);

    const oldProtocolInfo = await program.account.protocolVault.fetch(protocol);
    const olderSafetyAmount = oldProtocolInfo.safetyAmount;
    const olderYieldAmount = oldProtocolInfo.yieldAmount;

    // Add your test here.
    await program.methods
      .withdraw(deposit)
      .accountsStrict({
        operator: operator.publicKey,
        usdc: USDC,
        operatorAta: operatorAta,
        protocol: protocol,
        protocolAuthority: protocolAuthority,
        protocolAta: protocolAta,
        protocolKtokenAta: protocolKtokenAta,
        kaminoProgram: KAMINO_PROGRAM_ID,
        reserve: RESERVE,
        lendingMarket: LENDING_MARKET,
        lendingMarketAuthority: LENDING_MARKET_AUTHORITY,
        reserveLiquidityMint: USDC,
        reserveLiquiditySupply: RESERVE_LIQUIDITY_SUPPLY,
        reserveCollateralMint: RESERVE_COLLATERAL_MINT,

        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        instructionSysvar: SYSVAR_INSTRUCTIONS_PUBKEY,
      })
      .rpc();
    const protocolInfo = await program.account.protocolVault.fetch(protocol);
    expect(Number(protocolInfo.safetyAmount)).to.lessThan(Number(olderSafetyAmount));
    expect(Number(protocolInfo.yieldAmount)).to.lessThanOrEqual(Number(olderYieldAmount));

    const operatorInfo = await getAccount(provider.connection, operatorAta);
    expect(Number(operatorInfo.amount)).to.greaterThanOrEqual(Number(deposit));

  });



  it("Staff claim is complete!", async () => {
    // Add your test here.

    const oldstaffAccountInfo = await program.account.staffAccount.fetch(staffAccount);
    const oldertotalClaimed = oldstaffAccountInfo.totalClaimed;

    const oldStaffInfo = await getAccount(provider.connection, staffAta);
    const oldStaffAmount = oldStaffInfo.amount;

    const oldProtocolInfo = await program.account.protocolVault.fetch(protocol);
    const olderSafetyAmount = oldProtocolInfo.safetyAmount;
    const olderYieldAmount = oldProtocolInfo.yieldAmount;
    const olderLiability = oldProtocolInfo.liability;

    await program.methods
      .staffClaim()
      .accountsStrict({
        staff: staff.publicKey,
        usdc: USDC,
        staffAta: staffAta,
        staffAccount: staffAccount,
        protocol: protocol,
        protocolAuthority: protocolAuthority,
        protocolAta: protocolAta,
        
        protocolKtokenAta: protocolKtokenAta,
        kaminoProgram: KAMINO_PROGRAM_ID,
        reserve: RESERVE,
        lendingMarket: LENDING_MARKET,
        lendingMarketAuthority: LENDING_MARKET_AUTHORITY,
        reserveLiquidityMint: USDC,
        reserveLiquiditySupply: RESERVE_LIQUIDITY_SUPPLY,
        reserveCollateralMint: RESERVE_COLLATERAL_MINT,

        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        instructionSysvar: SYSVAR_INSTRUCTIONS_PUBKEY
      })
      .signers([staff])
      .rpc();
    const staffAccountInfo = await program.account.staffAccount.fetch(staffAccount);
    expect(Number(staffAccountInfo.totalClaimed)).to.greaterThanOrEqual(Number(oldertotalClaimed));

    const StaffAtaInfo = await getAccount(provider.connection, staffAta);
    expect(Number(StaffAtaInfo.amount)).to.greaterThanOrEqual(Number(oldStaffAmount));

    const protocolInfo = await program.account.protocolVault.fetch(protocol);
    expect(Number(protocolInfo.safetyAmount)).to.lessThanOrEqual(Number(olderSafetyAmount));
    expect(Number(protocolInfo.yieldAmount)).to.lessThanOrEqual(Number(olderYieldAmount));
    expect(Number(protocolInfo.liability)).to.lessThanOrEqual(Number(olderLiability));


  });




  it("Offbaording is complete!", async () => {
    // Add your test here.

    //const oldoperatorAta = await getAccount(provider.connection, operator.publicKey);

    const oldProtocolInfo = await program.account.protocolVault.fetch(protocol);
    const olderSafetyAmount = oldProtocolInfo.safetyAmount;
    const olderLiability = oldProtocolInfo.liability;
    const olderGlobalRate = oldProtocolInfo.globalRate;

    await program.methods
      .staffOffboard()
      .accountsStrict({
        operator: operator.publicKey,
        staff: staff.publicKey,
        usdc: USDC,
        staffAta: staffAta,
        staffAccount: staffAccount,
        protocol: protocol,
        protocolAuthority: protocolAuthority,
        protocolAta: protocolAta,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId
      })
      .rpc();

    //const operatorInfo = await getAccount(provider.connection, operator.publicKey);
    //expect(Number(operatorInfo.amount)).to.greaterThan(Number(oldoperatorAta));

    const protocolInfo = await program.account.protocolVault.fetch(protocol);
    expect(Number(protocolInfo.safetyAmount)).to.lessThanOrEqual(Number(olderSafetyAmount));
    expect(Number(protocolInfo.liability)).to.lessThanOrEqual(Number(olderLiability));
    expect(Number(protocolInfo.globalRate)).to.lessThan(Number(olderGlobalRate));

  });

});
