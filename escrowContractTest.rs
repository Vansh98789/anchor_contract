import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { EscrowContract } from "../target/types/escrow_contract";
import { assert } from "chai";
import { LAMPORTS_PER_SOL, SystemProgram } from "@solana/web3.js";

describe("escrow_contract", () => {
  const provider = anchor.AnchorProvider.local();
  anchor.setProvider(provider);

  const program = anchor.workspace.escrowContract as Program<EscrowContract>;
  const buyer = provider.wallet;
  const seller = anchor.web3.Keypair.generate();
  let escrowPda: anchor.web3.PublicKey;
  let bump: number;
  
  before(async () => {
    // Airdrop SOL to seller for testing
    await provider.connection.requestAirdrop(seller.publicKey, 2 * LAMPORTS_PER_SOL);
    
    // Derive the PDA for the escrow account (using buyer's public key and "escrow" as seed)
    [escrowPda, bump] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("escrow"), buyer.publicKey.toBuffer()],
      program.programId
    );
  });

  it("initialize escrow", async () => {
    // Initialize the escrow contract with the buyer, seller, and the escrow PDA
    await program.methods
      .initEscrow()
      .accounts({
        buyer: buyer.publicKey,
        seller: seller.publicKey,
      })
      .rpc();

    // Fetch the escrow account to verify the initialization
    const escrow = await program.account.escrowAccount.fetch(escrowPda);
    assert.ok(escrow.buyer.equals(buyer.publicKey), "Escrow buyer mismatch");
    assert.ok(escrow.seller.equals(seller.publicKey), "Escrow seller mismatch");
    assert.equal(escrow.amount.toNumber(), 0, "Escrow amount should be 0 after initialization");
  });

  it("depositing 1 sol in escrow", async () => {
    const depositAmount = 1 * LAMPORTS_PER_SOL;

    // Get the initial balance of the escrow PDA
    const initialBalance = await provider.connection.getBalance(escrowPda);

    // Perform the deposit transaction
    await program.methods
      .deposit(new anchor.BN(depositAmount))
      .accounts({
        buyer: buyer.publicKey,
      })
      .rpc();

    // Fetch the updated escrow account
    const escrow = await program.account.escrowAccount.fetch(escrowPda);
    
    // Check that the escrow account now holds the deposited amount
    assert.equal(escrow.amount.toNumber(), depositAmount, `Expected ${depositAmount}, got ${escrow.amount.toNumber()}`);
    
    // Optionally, check that the balance of the escrow PDA has increased
    const balanceAfterDeposit = await provider.connection.getBalance(escrowPda);
    assert.equal(balanceAfterDeposit, initialBalance + depositAmount, "Escrow balance mismatch after deposit");
  });

  it("release funds to seller", async () => {
    // Get the seller's balance before the release
    const balanceBeforeRelease = await provider.connection.getBalance(seller.publicKey);

    // Perform the release transaction
    await program.methods
      .release()
      .accounts({
        buyer: buyer.publicKey,
        seller: seller.publicKey,
      })
      .rpc();

    // Get the seller's balance after the release
    const balanceAfterRelease = await provider.connection.getBalance(seller.publicKey);

    // Check that the seller's balance has increased by the correct amount (1 SOL)
    assert.equal(balanceAfterRelease, balanceBeforeRelease + 1 * LAMPORTS_PER_SOL, "Seller balance mismatch after release");
  });
});
