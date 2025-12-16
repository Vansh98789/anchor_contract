use anchor_lang::prelude::*;
use anchor_lang::system_program; 

declare_id!("3xCUEPFY7c5kRMMRbrxM5ERQB5zNLvxdwRM2k4TuQuj4");

#[program]
pub mod market_place_contract {
    use super::*;

    pub fn create_listing(ctx: Context<CreateListing>, amount: u64, name: [u8; 32], item_type: ItemType, mint: Pubkey) -> Result<()> {
        require!(amount > 0, MarketError::InvalidAmount);
        let market_account = &mut ctx.accounts.listing_account;
        market_account.seller = ctx.accounts.seller.key();
        market_account.price = amount;
        market_account.item_details = ItemInfo {
            name,
            item_type,
            mint,
        };
        market_account.is_sold = false;
        market_account.bump = ctx.bumps.listing_account;
        Ok(())
    }

    pub fn buy(ctx: Context<Buy>) -> Result<()> {
        require!(!ctx.accounts.listing_account.is_sold, MarketError::AlreadySold);
        let listing = &mut ctx.accounts.listing_account;
        let cpi = CpiContext::new(
            ctx.accounts.system_program.to_account_info(), 
            system_program::Transfer {
                from: ctx.accounts.buyer.to_account_info(),
                to: listing.to_account_info(),
            }
        );
        system_program::transfer(cpi, listing.price)?;
        listing.is_sold = true;
        Ok(())
    }

    pub fn withdraw(ctx: Context<Withdraw>) -> Result<()> {
        require!(ctx.accounts.listing_account.is_sold, MarketError::NotSold);
        let listing = &mut ctx.accounts.listing_account;
        let amount = listing.price;
        **listing.to_account_info().try_borrow_mut_lamports()? -= amount;
        **ctx.accounts.seller.to_account_info().try_borrow_mut_lamports()? += amount;
        Ok(())
    }
}

#[derive(AnchorDeserialize, AnchorSerialize, Clone, Copy)]
pub struct ItemInfo { 
    pub name: [u8; 32],
    pub item_type: ItemType,
    pub mint: Pubkey,
}

#[derive(AnchorDeserialize, AnchorSerialize, Clone, Copy)]
pub enum ItemType {
    Physical,
    NFT,
    SPL,
}

#[account]
pub struct MarketListing {
    pub seller: Pubkey,
    pub price: u64,
    pub item_details: ItemInfo,
    pub is_sold: bool,
    pub bump: u8,
}

#[derive(Accounts)]
pub struct CreateListing<'info> {
    #[account(mut)]
    pub seller: Signer<'info>,
    #[account(
        init,
        payer = seller,
        space = 8 + 32 + 8 + 65 + 1 + 1,    
        seeds = [b"market", seller.key().as_ref()],
        bump
    )]
    pub listing_account: Account<'info, MarketListing>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
pub struct Buy<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,
    #[account(
        mut,
        // ⚠️ FIX: Use seller's key, not buyer's key!
        seeds = [b"market", listing_account.seller.as_ref()],
        bump = listing_account.bump,
    )]
    pub listing_account: Account<'info, MarketListing>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub seller: Signer<'info>,
    #[account(
        mut,
        seeds = [b"market", seller.key().as_ref()],
        bump = listing_account.bump,
    )]
    pub listing_account: Account<'info, MarketListing>,
    pub system_program: Program<'info, System>
}

#[error_code]
pub enum MarketError {
    #[msg("invalid amount")]
    InvalidAmount,
    #[msg("Already sold")]
    AlreadySold,
    #[msg("not yet sold")]
    NotSold,
}
