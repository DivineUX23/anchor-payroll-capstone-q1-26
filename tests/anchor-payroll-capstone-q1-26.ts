import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { AnchorPayrollCapstoneQ126 } from "../target/types/anchor_payroll_capstone_q1_26";
import { PublicKey, Keypair, SystemProgram, Ed25519Program, SYSVAR_INSTRUCTIONS_PUBKEY } from "@solana/web3.js";
import { expect } from "chai";
//import { ASSOCIATED_PROGRAM_ID, TOKEN_PROGRAM_ID } from "@coral-xyz/anchor/dist/cjs/utils/token";
import { getAssociatedTokenAddressSync, createAssociatedTokenAccountInstruction, createMint, mintTo, TOKEN_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID, getAccount } from "@solana/spl-token";

describe("anchor-payroll-capstone-q1-26", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env()
  anchor.setProvider(provider);

  const program = anchor.workspace.anchorPayrollCapstoneQ126 as Program<AnchorPayrollCapstoneQ126>;
  const connection = provider.connection;

  const operator = provider.wallet;
  const staff = Keypair.generate();
  const keeper = Keypair.generate();

  let usdc: PublicKey;
  let protocol: PublicKey;
  let staffAccount: PublicKey;
  let Bump: number;
  let instructionSysvar: PublicKey;

  before(async () => {
    await connection.requestAirdrop(staff.publicKey, 5_000_000_000);
    await connection.requestAirdrop(keeper.publicKey, 5_000_000_000);

    const seed1 = new anchor.BN(1111)
    protocol = PublicKey.findProgramAddressSync(
      [Buffer.from("protocol"), operator.publicKey.toBuffer(), seed1.toArrayLike(Buffer, "le", 16)],
      program.programId
    )[0];
  })

  it("Is initialized!", async () => {
    // Add your test here.

    const seed1 = new anchor.BN(1111);

    await program.methods
      .operatorInit(seed1)
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
      expect(protocolInfo.liabilityTimestamp).to.greaterThan(0);

  });


  it("Is initialized!", async () => {
    // Add your test here.
    const annualized_salary = new anchor.BN(5000);
    const seed1 = new anchor.BN(1111);

    await program.methods
      .staffInit(annualized_salary, seed1)
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
      expect(staffInfo.timeStarted).to.greaterThan(0);


      const protocolInfo = await program.account.protocolVault.fetch(protocol);
      expect(Number(protocolInfo.globalRate)).to.greaterThan(0);
      expect(Number(protocolInfo.liability)).to.greaterThan(0);


  });

});
