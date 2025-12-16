import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { MarketPlaceContract } from "../target/types/market_place_contract";
import { LAMPORTS_PER_SOL, SystemProgram, PublicKey } from "@solana/web3.js";
import { assert } from "chai";

describe("market_place_contract", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  
  const program = anchor.workspace.MarketPlaceContract as Program<MarketPlaceContract>;
  const seller = provider.wallet;
  const buyer = anchor.web3.Keypair.generate();
  
  let listingPda: PublicKey;

  before(async () => {
    // Airdrop SOL to buyer and wait for confirmation
    const signature = await provider.connection.requestAirdrop(
      buyer.publicKey,
      2 * LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction(signature);
    
    // Derive the listing PDA
    [listingPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("market"), seller.publicKey.toBuffer()],
      program.programId
    );
  });

  it("creates a listing PDA", async () => {
    const name = Buffer.from("Laptop".padEnd(32, "\0"));
    
    await program.methods
      .createListing(
        new anchor.BN(LAMPORTS_PER_SOL),
        [...name],
        { physical: {} },
        PublicKey.default
      )
      .accounts({
        seller: seller.publicKey,
      })
      .rpc();

    const listing = await program.account.marketListing.fetch(listingPda);
    
    assert.equal(listing.price.toNumber(), LAMPORTS_PER_SOL);
    assert.equal(listing.isSold, false);
    assert.equal(listing.seller.toBase58(), seller.publicKey.toBase58());
  });

  it("buys the listing", async () => {
    const balanceBefore = await provider.connection.getBalance(listingPda);

    await program.methods
      .buy()
      .accountsPartial({
        buyer: buyer.publicKey,
        listingAccount: listingPda,
        systemProgram: SystemProgram.programId,
      })
      .signers([buyer])
      .rpc();

    const balanceAfter = await provider.connection.getBalance(listingPda);
    const listing = await program.account.marketListing.fetch(listingPda);

    assert.equal(listing.isSold, true);
    assert.equal(balanceAfter, balanceBefore + LAMPORTS_PER_SOL);
  });

  it("withdraws SOL to seller", async () => {
    const sellerBalanceBefore = await provider.connection.getBalance(seller.publicKey);

    await program.methods
      .withdraw()
      .accountsPartial({
        seller: seller.publicKey,
        listingAccount: listingPda,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const sellerBalanceAfter = await provider.connection.getBalance(seller.publicKey);
    
    // Account for transaction fees
    assert.isAbove(sellerBalanceAfter, sellerBalanceBefore);
  });
});
