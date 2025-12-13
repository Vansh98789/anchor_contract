use anchor_lang::prelude::*;
use anchor_lang::system_program;

declare_id!("BjCuWasrQsLPcT9EpYHMBkFNR2sgtPNTvJpR7DD2PbV8");

#[program]
pub mod escrow_contract {
    use super::*;

    // -------------------------------
    // Initialize Escrow
    // -------------------------------
    pub fn init_escrow(ctx: Context<InitEscrow>) -> Result<()> {
        let escrow = &mut ctx.accounts.escrow_pda;
        escrow.buyer = ctx.accounts.buyer.key();
        escrow.seller = ctx.accounts.seller.key();
        escrow.amount = 0;
        escrow.state = EscrowState::Pending;
        escrow.bump = ctx.bumps.escrow_pda;

        Ok(())
    }

    // -------------------------------
    // Deposit SOL
    // -------------------------------
    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        require!(amount > 0, EscrowError::InvalidAmount);

        let escrow = &mut ctx.accounts.escrow_pda;

        // Transfer SOL from buyer to PDA
        let cpi = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: ctx.accounts.buyer.to_account_info(),
                to: escrow.to_account_info(),
            },
        );
        system_program::transfer(cpi, amount)?;

        escrow.amount = escrow
            .amount
            .checked_add(amount)
            .ok_or(EscrowError::AmountOverflow)?;
        escrow.state = EscrowState::BuyerDeposit;

        Ok(())
    }

    // -------------------------------
    // Release SOL to seller
    // -------------------------------
    pub fn release(ctx: Context<Release>) -> Result<()> {
        let escrow = &mut ctx.accounts.escrow_pda;

        require!(escrow.amount > 0, EscrowError::InvalidAmount);
       require!(escrow.state == EscrowState::BuyerDeposit, EscrowError::InvalidState);

        let seeds = &[b"escrow", escrow.buyer.as_ref(), &[escrow.bump]];
        let signer = &[&seeds[..]];

        let cpi = CpiContext::new_with_signer(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: escrow.to_account_info(),
                to: ctx.accounts.seller.to_account_info(),
            },
            signer,
        );

        system_program::transfer(cpi, escrow.amount)?;
        escrow.amount = 0;
        escrow.state = EscrowState::Completed;

        Ok(())
    }

    // -------------------------------
    // Refund SOL to buyer
    // -------------------------------
    pub fn refund(ctx: Context<Refund>) -> Result<()> {
        let escrow = &mut ctx.accounts.escrow_pda;

        require!(escrow.amount > 0, EscrowError::InvalidAmount);
        require!(escrow.state == EscrowState::BuyerDeposit, EscrowError::InvalidState);


        let seeds = &[b"escrow", escrow.buyer.as_ref(), &[escrow.bump]];
        let signer = &[&seeds[..]];

        let cpi = CpiContext::new_with_signer(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: escrow.to_account_info(),
                to: ctx.accounts.buyer.to_account_info(),
            },
            signer,
        );

        system_program::transfer(cpi, escrow.amount)?;
        escrow.amount = 0;
        escrow.state = EscrowState::Cancelled;

        Ok(())
    }
}

// -------------------------------
// Escrow State
// -------------------------------
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum EscrowState {
    Pending,
    BuyerDeposit,
    Completed,
    Cancelled,
}

// -------------------------------
// Escrow Account
// -------------------------------
#[account]
pub struct EscrowAccount {
    pub buyer: Pubkey,
    pub seller: Pubkey,
    pub amount: u64,
    pub state: EscrowState,
    pub bump: u8,
}

// -------------------------------
// Accounts Context
// -------------------------------
#[derive(Accounts)]
pub struct InitEscrow<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,
    /// CHECK: Seller is just a pubkey
    pub seller: SystemAccount<'info>,

    #[account(
        init,
        payer = buyer,
        space = 8 + 32 + 32 + 8 + 1 + 1,
        seeds = [b"escrow", buyer.key().as_ref()],
        bump
    )]
    pub escrow_pda: Account<'info, EscrowAccount>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,

    #[account(mut, seeds = [b"escrow", buyer.key().as_ref()], bump = escrow_pda.bump)]
    pub escrow_pda: Account<'info, EscrowAccount>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Release<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,

    /// CHECK: Seller receives SOL
    #[account(mut)]
    pub seller: SystemAccount<'info>,

    #[account(mut, seeds = [b"escrow", buyer.key().as_ref()], bump = escrow_pda.bump)]
    pub escrow_pda: Account<'info, EscrowAccount>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Refund<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,

    #[account(mut, seeds = [b"escrow", buyer.key().as_ref()], bump = escrow_pda.bump)]
    pub escrow_pda: Account<'info, EscrowAccount>,

    pub system_program: Program<'info, System>,
}

// -------------------------------
// Errors
// -------------------------------
#[error_code]
pub enum EscrowError {
    #[msg("Invalid amount")]
    InvalidAmount,
    #[msg("Amount overflow")]
    AmountOverflow,
    #[msg("Invalid state")]
    InvalidState,
}
