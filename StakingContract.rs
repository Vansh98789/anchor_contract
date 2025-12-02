use anchor_lang::prelude::*;
use anchor_lang::system_program;
use anchor_spl::token::{self, Mint, Token, TokenAccount, MintTo};

declare_id!("4DGD5eBb5SXfSKoEggTjDTw6CSvYg97V6CoVjT34zK9W");

#[program]
pub mod staking_contract {
    use super::*;

    pub fn init(ctx: Context<Init>) -> Result<()> {
        let pda = &mut ctx.accounts.pda_account;

        pda.owner = ctx.accounts.payer.key();
        pda.stake_amount = 0;
        pda.total_points = 0;
        pda.last_update = Clock::get()?.unix_timestamp;
        pda.reward_mint = ctx.accounts.reward_mint.key();
        pda.bump = *ctx.bumps.get("pda_account").unwrap();

        msg!("Staking account initialized.");
        Ok(())
    }

    pub fn stake(ctx: Context<Stake>, amount: u64) -> Result<()> {
        require!(amount > 0, StakeError::InvalidAmount);

        let pda = &mut ctx.accounts.pda_account;
        let clock = Clock::get()?;

        update_points(pda, clock.unix_timestamp)?;

        // Transfer lamports user → PDA
        let cpi = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: ctx.accounts.user.to_account_info(),
                to: ctx.accounts.pda_account.to_account_info(),
            },
        );
        system_program::transfer(cpi, amount)?;

        pda.stake_amount = pda
            .stake_amount
            .checked_add(amount)
            .ok_or(StakeError::Overflow)?;

        Ok(())
    }

    pub fn unstake(ctx: Context<Unstake>, amount: u64) -> Result<()> {
        require!(amount > 0, StakeError::InvalidAmount);

        let pda = &mut ctx.accounts.pda_account;
        let clock = Clock::get()?;

        require!(pda.stake_amount >= amount, StakeError::InsufficientStakeBalance);

        update_points(pda, clock.unix_timestamp)?;

        // PDA signer seeds
        let seeds = &[
            b"client1",
            ctx.accounts.user.key.as_ref(),
            &[pda.bump],
        ];
        let signer = &[&seeds[..]];

        // Transfer lamports PDA → user
        let cpi = CpiContext::new_with_signer(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: ctx.accounts.pda_account.to_account_info(),
                to: ctx.accounts.user.to_account_info(),
            },
            signer,
        );
        system_program::transfer(cpi, amount)?;

        pda.stake_amount = pda
            .stake_amount
            .checked_sub(amount)
            .ok_or(StakeError::Overflow)?;

        Ok(())
    }

    pub fn claim_points(ctx: Context<ClaimPoints>) -> Result<()> {
        let pda = &mut ctx.accounts.pda_account;
        let clock = Clock::get()?;

        update_points(pda, clock.unix_timestamp)?;

        // Convert points → reward tokens
        let claimable_points = pda.total_points / 1_000_000;
        require!(claimable_points > 0, StakeError::NothingToClaim);

        msg!("User can claim {} reward tokens.", claimable_points);

        // PDA (mint authority) signer
        let seeds = &[
            b"client1",
            ctx.accounts.user.key.as_ref(),
            &[pda.bump],
        ];
        let signer = &[&seeds[..]];

        // Mint tokens to user
        let mint_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            MintTo {
                mint: ctx.accounts.reward_mint.to_account_info(),
                to: ctx.accounts.user_token_account.to_account_info(),
                authority: ctx.accounts.pda_account.to_account_info(),
            },
            signer,
        );

        token::mint_to(mint_ctx, claimable_points as u64)?;

        pda.total_points = 0;

        Ok(())
    }
}


/// --------------------------------------
/// INTERNAL POINT CALCULATION FUNCTION
/// --------------------------------------
fn update_points(pda: &mut StakeAccount, current_time: i64) -> Result<()> {
    let time_elapsed = current_time
        .checked_sub(pda.last_update)
        .ok_or(StakeError::InvalidTimestamp)?;

    if time_elapsed > 0 && pda.stake_amount > 0 {
        let new_points = (pda.stake_amount as u128)
            .checked_mul(time_elapsed as u128)
            .ok_or(StakeError::Overflow)?;

        pda.total_points = pda
            .total_points
            .checked_add(new_points)
            .ok_or(StakeError::Overflow)?;
    }

    pda.last_update = current_time;
    Ok(())
}


/// --------------------------------------
/// ACCOUNT STRUCTS
/// --------------------------------------
#[account]
pub struct StakeAccount {
    pub owner: Pubkey,
    pub stake_amount: u64,
    pub total_points: u128,
    pub last_update: i64,
    pub reward_mint: Pubkey,   // <── NEW: reward token mint
    pub bump: u8,
}


/// --------------------------------------
/// ACCOUNT CONTEXTS
/// --------------------------------------

#[derive(Accounts)]
pub struct Init<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    // Reward mint (owner controls supply initially)
    #[account(mut)]
    pub reward_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = payer,
        space = 8 + 32 + 8 + 16 + 8 + 32 + 1,
        seeds = [b"client1", payer.key().as_ref()],
        bump
    )]
    pub pda_account: Account<'info, StakeAccount>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct Stake<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [b"client1", user.key().as_ref()],
        bump = pda_account.bump
    )]
    pub pda_account: Account<'info, StakeAccount>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Unstake<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [b"client1", user.key().as_ref()],
        bump = pda_account.bump
    )]
    pub pda_account: Account<'info, StakeAccount>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ClaimPoints<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [b"client1", user.key().as_ref()],
        bump = pda_account.bump
    )]
    pub pda_account: Account<'info, StakeAccount>,

    #[account(mut)]
    pub reward_mint: Account<'info, Mint>,

    #[account(
        mut,
        constraint = user_token_account.owner == user.key(),
        constraint = user_token_account.mint == reward_mint.key()
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}


/// --------------------------------------
/// ERRORS
/// --------------------------------------
#[error_code]
pub enum StakeError {
    #[msg("Invalid staking amount")]
    InvalidAmount,
    #[msg("Nothing to claim")]
    NothingToClaim,
    #[msg("Insufficient staked balance")]
    InsufficientStakeBalance,
    #[msg("Overflow error")]
    Overflow,
    #[msg("Timestamp error")]
    InvalidTimestamp,
}
