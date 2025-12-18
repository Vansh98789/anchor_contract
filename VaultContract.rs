use anchor_lang::prelude::*;
use anchor_lang::system_program;

declare_id!("2Hzb64AtazgV7yxkxgVQjUgWnr7X1V5s9D55cSSgiJbY");

#[program]
pub mod vault_contract {
    use super::*;

    /// Initialize the shared vault
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        vault.owner = ctx.accounts.owner.key();
        vault.total_deposited = 0;
        vault.total_shares = 0;
        vault.bump = ctx.bumps.vault;
        
        msg!("Vault initialized by owner: {}", vault.owner);
        Ok(())
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        require!(amount > 0, VaultError::InvalidAmount);

        let cpi_context = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: ctx.accounts.depositor.to_account_info(),
                to: ctx.accounts.vault.to_account_info(),
            },
        );
        system_program::transfer(cpi_context, amount)?;

        let vault = &mut ctx.accounts.vault;
        vault.total_deposited = vault.total_deposited
            .checked_add(amount)
            .ok_or(VaultError::AmountOverflow)?;
        
        vault.total_shares = vault.total_shares
            .checked_add(amount)
            .ok_or(VaultError::AmountOverflow)?;

        let user_deposit = &mut ctx.accounts.user_deposit;
        user_deposit.user = ctx.accounts.depositor.key();
        user_deposit.vault = vault.key();
        user_deposit.shares = user_deposit.shares
            .checked_add(amount)
            .ok_or(VaultError::AmountOverflow)?;
        user_deposit.bump = ctx.bumps.user_deposit;

        msg!("Deposited {} lamports. User now has {} shares", amount, user_deposit.shares);
        Ok(())
    }

    pub fn withdraw(ctx: Context<Withdraw>) -> Result<()> {
        let user_deposit = &ctx.accounts.user_deposit;
        let vault = &mut ctx.accounts.vault;

        require!(user_deposit.shares > 0, VaultError::NoDeposit);

        let vault_balance = vault.to_account_info().lamports();
        
      
        let user_share = (user_deposit.shares as u128)
            .checked_mul(vault_balance as u128)
            .ok_or(VaultError::AmountOverflow)?
            .checked_div(vault.total_shares as u128)
            .ok_or(VaultError::DivisionByZero)?;

        require!(user_share <= u64::MAX as u128, VaultError::AmountOverflow);
        let user_share = user_share as u64;

        let owner_fee = user_share
            .checked_mul(2)
            .ok_or(VaultError::AmountOverflow)?
            .checked_div(100)
            .ok_or(VaultError::DivisionByZero)?;

        let net_withdrawal = user_share
            .checked_sub(owner_fee)
            .ok_or(VaultError::InsufficientFunds)?;

        require!(
            vault_balance >= user_share,
            VaultError::InsufficientFunds
        );

        // Transfer net amount to user
        **vault.to_account_info().try_borrow_mut_lamports()? -= net_withdrawal;
        **ctx.accounts.depositor.to_account_info().try_borrow_mut_lamports()? += net_withdrawal;

        // Transfer fee to owner
        **vault.to_account_info().try_borrow_mut_lamports()? -= owner_fee;
        **ctx.accounts.owner.to_account_info().try_borrow_mut_lamports()? += owner_fee;

        vault.total_shares = vault.total_shares
            .checked_sub(user_deposit.shares)
            .ok_or(VaultError::AmountOverflow)?;
        
        vault.total_deposited = vault.total_deposited
            .checked_sub(user_deposit.shares)
            .ok_or(VaultError::AmountOverflow)?;

        msg!(
            "Withdrawn {} lamports (fee: {} lamports). Profit: {} lamports",
            net_withdrawal,
            owner_fee,
            user_share.saturating_sub(user_deposit.shares)
        );

        Ok(())
    }

    pub fn owner_withdraw(ctx: Context<OwnerWithdraw>, amount: u64) -> Result<()> {
        let vault = &ctx.accounts.vault;
        let vault_balance = vault.to_account_info().lamports();

        require!(amount > 0, VaultError::InvalidAmount);
        require!(vault_balance >= amount, VaultError::InsufficientFunds);

        let seeds = &[b"vault".as_ref(), &[vault.bump]];
        let signer = &[&seeds[..]];

        let cpi_context = CpiContext::new_with_signer(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: vault.to_account_info(),
                to: ctx.accounts.owner.to_account_info(),
            },
            signer,
        );
        system_program::transfer(cpi_context, amount)?;

        msg!("Owner withdrew {} lamports", amount);
        Ok(())
    }
}


#[account]
pub struct Vault {
    pub owner: Pubkey,           
    pub total_deposited: u64,    
    pub total_shares: u64,       
    pub bump: u8,
}

#[account]
pub struct UserDeposit {
    pub user: Pubkey,    
    pub vault: Pubkey,   
    pub shares: u64,     
    pub bump: u8,        
}


#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    
    #[account(
        init,
        payer = owner,
        space = 8 + 32 + 8 + 8 + 1,
        seeds = [b"vault"],
        bump
    )]
    pub vault: Account<'info, Vault>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub depositor: Signer<'info>,
    
    #[account(
        mut,
        seeds = [b"vault"],
        bump = vault.bump
    )]
    pub vault: Account<'info, Vault>,
    
    #[account(
        mut,
        seeds = [b"deposit", vault.key().as_ref(), depositor.key().as_ref()],
        bump
    )]
    pub user_deposit: Account<'info, UserDeposit>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub depositor: Signer<'info>,
    
    #[account(
        mut,
        seeds = [b"vault"],
        bump = vault.bump
    )]
    pub vault: Account<'info, Vault>,
    
    #[account(
        mut,
        seeds = [b"deposit", vault.key().as_ref(), depositor.key().as_ref()],
        bump = user_deposit.bump,
        close = depositor
    )]
    pub user_deposit: Account<'info, UserDeposit>,
    
    /// CHECK: Owner account to receive fees
    #[account(mut, constraint = owner.key() == vault.owner)]
    pub owner: AccountInfo<'info>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct OwnerWithdraw<'info> {
    #[account(mut, constraint = owner.key() == vault.owner @ VaultError::Unauthorized)]
    pub owner: Signer<'info>,
    
    #[account(
        mut,
        seeds = [b"vault"],
        bump = vault.bump
    )]
    pub vault: Account<'info, Vault>,
    
    pub system_program: Program<'info, System>,
}

#[error_code]
pub enum VaultError {
    #[msg("Invalid amount to deposit or withdraw")]
    InvalidAmount,
    
    #[msg("Amount overflow error")]
    AmountOverflow,
    
    #[msg("User has no deposit in the vault")]
    NoDeposit,
    
    #[msg("Insufficient funds in vault")]
    InsufficientFunds,
    
    #[msg("Division by zero")]
    DivisionByZero,
    
    #[msg("Unauthorized: only owner can perform this action")]
    Unauthorized,
}
