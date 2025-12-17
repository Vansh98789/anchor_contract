use anchor_lang::prelude::*;

declare_id!("BXxXhYtpCNDjQdAL5rsxipveEfcb6mTufRpDJc5hXQh6");

#[program]
pub mod governance_contract {
    use super::*;
    
    pub fn init_dao(ctx: Context<InitializeDao>) -> Result<()> {
        let dao = &mut ctx.accounts.dao;
        dao.authority = ctx.accounts.authority.key();
        dao.count = 0;
        Ok(())
    }
    
    pub fn create_proposal(
        ctx: Context<CreateProposal>,
        title: String,
        description: String
    ) -> Result<()> {
        require!(title.len() <= 100, GovtError::TitleTooLong);
        require!(description.len() <= 500, GovtError::DescriptionTooLong);
        
        let dao = &mut ctx.accounts.dao;
        let proposal = &mut ctx.accounts.proposal;
        
        proposal.id = dao.count;
        proposal.title = title;
        proposal.description = description;
        proposal.yes_vote = 0;
        proposal.no_vote = 0;
        proposal.executed = false;
        
        dao.count += 1;
        Ok(())
    }
    
    pub fn vote(ctx: Context<Vote>, approve: bool) -> Result<()> {
        let proposal = &mut ctx.accounts.proposal;
        let vote_record = &mut ctx.accounts.vote_record;
        
        require!(!vote_record.voted, GovtError::AlreadyVoted);
        require!(!proposal.executed, GovtError::ProposalAlreadyExecuted);
        
        if approve {
            proposal.yes_vote += 1;
        } else {
            proposal.no_vote += 1;
        }
        
        vote_record.voted = true;
        Ok(())
    }
    
    pub fn finalize(ctx: Context<Finalize>) -> Result<()> {
        let proposal = &mut ctx.accounts.proposal;
        
        require!(!proposal.executed, GovtError::ProposalAlreadyExecuted);
        require!(proposal.yes_vote > proposal.no_vote, GovtError::NoExecution);
        
        proposal.executed = true;
        Ok(())
    }
}

#[account]
pub struct Dao {
    pub authority: Pubkey,  // 32
    pub count: u64,         // 8
}

#[account]
pub struct Proposal {
    pub id: u64,              // 8
    pub title: String,        // 4 + 100
    pub description: String,  // 4 + 500
    pub yes_vote: u64,        // 8
    pub no_vote: u64,         // 8
    pub executed: bool,       // 1
}

#[account]
pub struct VoteRecord {
    pub voted: bool,  // 1
}

#[derive(Accounts)]
pub struct InitializeDao<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + 32 + 8
    )]
    pub dao: Account<'info, Dao>,
    
    #[account(mut)]
    pub authority: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CreateProposal<'info> {
    #[account(mut)]
    pub dao: Account<'info, Dao>,
    
    #[account(
        init,
        payer = proposer,
        space = 8 + 8 + (4 + 100) + (4 + 500) + 8 + 8 + 1
    )]
    pub proposal: Account<'info, Proposal>,
    
    #[account(mut)]
    pub proposer: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Vote<'info> {
    #[account(mut)]
    pub proposal: Account<'info, Proposal>,
    
    #[account(
        init,
        payer = voter,
        space = 8 + 1,
        seeds = [b"vote", proposal.key().as_ref(), voter.key().as_ref()],
        bump,
    )]
    pub vote_record: Account<'info, VoteRecord>,
    
    #[account(mut)]
    pub voter: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Finalize<'info> {
    #[account(mut)]
    pub proposal: Account<'info, Proposal>,
    
    // Optional: Add authority constraint
    // pub authority: Signer<'info>,
    // #[account(has_one = authority)]
    // pub dao: Account<'info, Dao>,
}

#[error_code]
pub enum GovtError {
    #[msg("Already voted on this proposal")]
    AlreadyVoted,
    
    #[msg("Proposal cannot be executed - not enough votes")]
    NoExecution,
    
    #[msg("Proposal has already been executed")]
    ProposalAlreadyExecuted,
    
    #[msg("Title is too long (max 100 characters)")]
    TitleTooLong,
    
    #[msg("Description is too long (max 500 characters)")]
    DescriptionTooLong,
}
