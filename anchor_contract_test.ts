import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { GovernanceContract } from "../target/types/governance_contract";
import { LAMPORTS_PER_SOL } from "@solana/web3.js";
import { assert } from "chai";

describe("governance_contract", () => {
  const provider = anchor.AnchorProvider.local();
  anchor.setProvider(provider);

  const program = anchor.workspace.governanceContract as Program<GovernanceContract>;
  
  let dao = anchor.web3.Keypair.generate();
  let proposal = anchor.web3.Keypair.generate();
  let voter1 = anchor.web3.Keypair.generate();
  let voter2 = anchor.web3.Keypair.generate();

  before(async () => {
    // Airdrop SOL to voters
    const airdrop1 = await provider.connection.requestAirdrop(
      voter1.publicKey,
      5 * LAMPORTS_PER_SOL
    );
    const airdrop2 = await provider.connection.requestAirdrop(
      voter2.publicKey,
      5 * LAMPORTS_PER_SOL
    );
    
    // Wait for confirmations
    await provider.connection.confirmTransaction(airdrop1);
    await provider.connection.confirmTransaction(airdrop2);
  });

  it("Initializes the DAO", async () => {
    await program.methods
      .initDao()
      .accounts({
        dao: dao.publicKey,
        authority: provider.wallet.publicKey,
      })
      .signers([dao])
      .rpc();

    const daoAccount = await program.account.dao.fetch(dao.publicKey);
    assert.equal(daoAccount.count.toNumber(), 0);
    assert.equal(
      daoAccount.authority.toBase58(),
      provider.wallet.publicKey.toBase58()
    );
  });

  it("Creates a proposal", async () => {
    await program.methods
      .createProposal("Proposal 1", "Accept the proposal please")
      .accounts({
        dao: dao.publicKey,
        proposal: proposal.publicKey,
        proposer: provider.wallet.publicKey,
      })
      .signers([proposal])
      .rpc();

    const proposalAccount = await program.account.proposal.fetch(proposal.publicKey);
    assert.equal(proposalAccount.title, "Proposal 1");
    assert.equal(proposalAccount.description, "Accept the proposal please");
    assert.equal(proposalAccount.yesVote.toNumber(), 0);
    assert.equal(proposalAccount.noVote.toNumber(), 0);
    assert.equal(proposalAccount.executed, false);

    // Verify DAO count incremented
    const daoAccount = await program.account.dao.fetch(dao.publicKey);
    assert.equal(daoAccount.count.toNumber(), 1);
  });

  it("Voter 1 votes YES", async () => {
    // Don't pass voteRecord - Anchor derives it automatically
    await program.methods
      .vote(true)
      .accounts({
        proposal: proposal.publicKey,
        voter: voter1.publicKey,
      })
      .signers([voter1])
      .rpc();

    const proposalAccount = await program.account.proposal.fetch(proposal.publicKey);
    assert.equal(proposalAccount.yesVote.toNumber(), 1);
    assert.equal(proposalAccount.noVote.toNumber(), 0);
  });

  it("Voter 2 votes NO", async () => {
    await program.methods
      .vote(false)
      .accounts({
        proposal: proposal.publicKey,
        voter: voter2.publicKey,
      })
      .signers([voter2])
      .rpc();

    const proposalAccount = await program.account.proposal.fetch(proposal.publicKey);
    assert.equal(proposalAccount.yesVote.toNumber(), 1);
    assert.equal(proposalAccount.noVote.toNumber(), 1);
  });

  it("Prevents double voting", async () => {
    try {
      await program.methods
        .vote(true)
        .accounts({
          proposal: proposal.publicKey,
          voter: voter1.publicKey,
        })
        .signers([voter1])
        .rpc();
      
      assert.fail("Should have thrown error for double voting");
    } catch (err) {
      assert.ok(err);
    }
  });

  it("Cannot finalize with tie votes", async () => {
    try {
      await program.methods
        .finalize()
        .accounts({
          proposal: proposal.publicKey,
        })
        .rpc();
      
      assert.fail("Should not finalize with equal votes");
    } catch (err) {
      assert.include(err.toString(), "NoExecution");
    }
  });

  it("Finalizes proposal after YES wins", async () => {
    // Create new proposal
    let newProposal = anchor.web3.Keypair.generate();
    
    await program.methods
      .createProposal("Proposal 2", "This will pass")
      .accounts({
        dao: dao.publicKey,
        proposal: newProposal.publicKey,
        proposer: provider.wallet.publicKey,
      })
      .signers([newProposal])
      .rpc();

    // Vote YES with voter1
    await program.methods
      .vote(true)
      .accounts({
        proposal: newProposal.publicKey,
        voter: voter1.publicKey,
      })
      .signers([voter1])
      .rpc();

    // Finalize
    await program.methods
      .finalize()
      .accounts({
        proposal: newProposal.publicKey,
      })
      .rpc();

    const proposalAccount = await program.account.proposal.fetch(newProposal.publicKey);
    assert.equal(proposalAccount.executed, true);
  });
});
