import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { StakingContract } from "../target/types/staking_contract";
import {
  createMint,
  getOrCreateAssociatedTokenAccount,
  getAccount,
} from "@solana/spl-token";
import { assert } from "chai";

describe("staking_contract", () => {
  const provider = anchor.AnchorProvider.local();
  anchor.setProvider(provider);
  const program = anchor.workspace.StakingContract as Program<StakingContract>;
  const payer = provider.wallet;
  
  let rewardMint: anchor.web3.PublicKey;
  let userTokenAccount: anchor.web3.PublicKey;
  let pdaAccount: anchor.web3.PublicKey;
  let bump: number;
  const seed = Buffer.from("client1");

  it("Initialize reward mint and PDA first", async () => {
    // Calculate PDA first
    [pdaAccount, bump] = anchor.web3.PublicKey.findProgramAddressSync(
      [seed, payer.publicKey.toBuffer()],
      program.programId
    );

    // Create mint with PDA as the mint authority
    rewardMint = await createMint(
      provider.connection,
      payer.payer,
      pdaAccount, // PDA is the mint authority
      null,
      6 // decimals
    );

    userTokenAccount = (
      await getOrCreateAssociatedTokenAccount(
        provider.connection,
        payer.payer,
        rewardMint,
        payer.publicKey
      )
    ).address;

    assert.ok(rewardMint, "Reward mint should be created");
    assert.ok(userTokenAccount, "User token account should be created");
  });

  it("Initialize staking PDA", async () => {
    await program.methods
      .init()
      .accounts({
        payer: payer.publicKey,
        rewardMint,
        pdaAccount,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
      })
      .rpc();

    const pdaInfo = await provider.connection.getAccountInfo(pdaAccount);
    assert.ok(pdaInfo, "PDA account should exist after init");
  });

  it("Stake 0.1 SOL", async () => {
    const amount = anchor.web3.LAMPORTS_PER_SOL / 10;
    
    // Get balance before staking
    const pdaBalanceBefore = await provider.connection.getBalance(pdaAccount);

    await program.methods
      .stake(new anchor.BN(amount))
      .accounts({
        payer: payer.publicKey,
        pdaAccount,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    // Get balance after staking
    const pdaBalanceAfter = await provider.connection.getBalance(pdaAccount);
    const stakedAmount = pdaBalanceAfter - pdaBalanceBefore;

    assert.equal(
      stakedAmount,
      amount,
      `Expected ${amount} lamports to be staked, but got ${stakedAmount}`
    );
  });

  it("Wait 2 seconds to accumulate points", async () => {
    await new Promise((resolve) => setTimeout(resolve, 2000));
  });

  it("Claim rewards (mint tokens)", async () => {
    await program.methods
      .claimPoints()
      .accounts({
        payer: payer.publicKey,
        pdaAccount,
        rewardMint,
        userTokenAccount,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
      })
      .rpc();

    const account = await getAccount(provider.connection, userTokenAccount);
    const userBalance = Number(account.amount) / 10 ** 6;
    
    console.log(`User reward balance: ${userBalance} tokens`);
    assert.ok(userBalance > 0, "User reward token balance should be greater than zero");
  });

  it("Unstake 0.05 SOL", async () => {
    const unstakeAmount = anchor.web3.LAMPORTS_PER_SOL / 20;
    
    // Get balances before unstaking
    const pdaBalanceBefore = await provider.connection.getBalance(pdaAccount);
    const payerBalanceBefore = await provider.connection.getBalance(payer.publicKey);

    await program.methods
      .unstake(new anchor.BN(unstakeAmount))
      .accounts({
        payer: payer.publicKey,
        pdaAccount,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    // Get balances after unstaking
    const pdaBalanceAfter = await provider.connection.getBalance(pdaAccount);
    const payerBalanceAfter = await provider.connection.getBalance(payer.publicKey);

    const pdaDecrease = pdaBalanceBefore - pdaBalanceAfter;
    
    // Check PDA lost the unstaked amount
    assert.equal(
      pdaDecrease,
      unstakeAmount,
      `Expected PDA to decrease by ${unstakeAmount}, but decreased by ${pdaDecrease}`
    );

    // Check payer received SOL (accounting for tx fees)
    const payerIncrease = payerBalanceAfter - payerBalanceBefore;
    assert.ok(
      payerIncrease > 0,
      `Payer should have received SOL. Balance change: ${payerIncrease}`
    );
  });
});
