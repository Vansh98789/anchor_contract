use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};
declare_id!("AvgpkY8zisAoyqsnxmqSaennBpBVkan7bpd2LgJXQEGx");
// tokena/token b
//user -> token a
//user -> token b
// lp_mint -> lp_token
// add liquidity
// remove liquidity
// swap 
// lp_token
#[program]
pub mod amm_contract {
    use anchor_spl::token::{self, Burn, MintTo, Transfer};
    use super::*;
    
    pub fn initialize(ctx:Context<Initialize>)->Result<()>{
        let pool=&mut ctx.accounts.pool;
        pool.token_a_mint=ctx.accounts.token_a_mint.key();
        pool.token_b_mint=ctx.accounts.token_b_mint.key();
        pool.fees_percentage=30;
        pool.bump=ctx.bumps.pool;
        Ok(())
    }
    
    pub fn deposite(ctx:Context<Deposite>,amount_a:u64,amount_b:u64)->Result<()>{
        require!(amount_a>0,AMMErrors::InvalidAmount);
        require!(amount_b>0,AMMErrors::InvalidAmount);
        
        let vault_a=&ctx.accounts.vault_a;
        let vault_b=&ctx.accounts.vault_b;
        let lp_mint=&ctx.accounts.lp_mint;
        
        let lp_amount=if lp_mint.supply==0{
            ((amount_a as u128 *amount_b as u128).integer_sqrt()) as u64
        }
        else{
            let lp_from_a = (amount_a as u128 * lp_mint.supply as u128 / vault_a.amount as u128) as u64;
            let lp_from_b = (amount_b as u128 * lp_mint.supply as u128 / vault_b.amount as u128) as u64;
            std::cmp::min(lp_from_a, lp_from_b)
        };
        
        let cpi=CpiContext::new(
            ctx.accounts.token_program.to_account_info(), 
            Transfer{
                from:ctx.accounts.user_token_a.to_account_info(),
                to:vault_a.to_account_info(),
                authority:ctx.accounts.payer.to_account_info(),
            });
        token::transfer(cpi, amount_a)?;
        
        let cpi_a=CpiContext::new(
            ctx.accounts.token_program.to_account_info(), 
            Transfer{
                from:ctx.accounts.user_token_b.to_account_info(),
                to:vault_b.to_account_info(),
                authority:ctx.accounts.payer.to_account_info(),
            });
        token::transfer(cpi_a, amount_b)?;
        let payer_key=ctx.accounts.payer.key();
        let seeds=&[b"pool",payer_key.as_ref(), &[ctx.accounts.pool.bump]];
        let signer=&[&seeds[..]];
        
        let cpi_aa=CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),        
            MintTo{
                mint:lp_mint.to_account_info(),
                to:ctx.accounts.user_token_lp.to_account_info(),
                authority:ctx.accounts.pool.to_account_info(),
            }, signer);
        token::mint_to(cpi_aa, lp_amount)?;
        Ok(())
    }
    
    pub fn remove_liquidity(ctx:Context<RemoveLiquidity>,amount:u64)->Result<()> {
        require!(amount>0,AMMErrors::InvalidAmount);
        
        let vault_a=&ctx.accounts.vault_a;
        let vault_b=&ctx.accounts.vault_b;
        let lp_mint=&ctx.accounts.lp_mint;
        
        let amount_a=(amount as u128 * vault_a.amount as u128 / lp_mint.supply as u128) as u64;
        let amount_b=(amount as u128 * vault_b.amount as u128 / lp_mint.supply as u128) as u64;
        
        let cpi_burn=CpiContext::new(
            ctx.accounts.token_program.to_account_info(), 
            Burn{
                mint: ctx.accounts.lp_mint.to_account_info(),
                from: ctx.accounts.user_token_lp.to_account_info(),
                authority: ctx.accounts.payer.to_account_info()
            }
        );
        token::burn(cpi_burn, amount)?;
        let payer_key=ctx.accounts.payer.key();
        let seeds=&[b"pool",payer_key.as_ref(), &[ctx.accounts.pool.bump]];
        let signer=&[&seeds[..]];
        
        let cpi_aa=CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),        
            Transfer{
                from:vault_a.to_account_info(),
                to:ctx.accounts.user_token_a.to_account_info(),
                authority:ctx.accounts.pool.to_account_info(),
            }, signer);
        token::transfer(cpi_aa, amount_a)?;
        
        let cpi_ab=CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),        
            Transfer{
                from:vault_b.to_account_info(),
                to:ctx.accounts.user_token_b.to_account_info(),
                authority:ctx.accounts.pool.to_account_info(),
            }, signer);
        token::transfer(cpi_ab, amount_b)?;
        
        Ok(())
    }
    
    pub fn swap(ctx:Context<Swap>,amount:u64,is_a_to_b:bool)->Result<()>{
        require!(amount>0,AMMErrors::InvalidAmount);
        
        let vault_a=&ctx.accounts.vault_a;
        let vault_b=&ctx.accounts.vault_b;
        
        let (reserve_in, reserve_out) = if is_a_to_b {
            (vault_a.amount, vault_b.amount)
        } else {
            (vault_b.amount, vault_a.amount)
        };
        
        let amount_in_after_fee = (amount as u128 * 9970) / 10000;
        let amount_out = (amount_in_after_fee * reserve_out as u128) 
            / (reserve_in as u128 + amount_in_after_fee);
        let amount_out = amount_out as u64;
        let payer_key=ctx.accounts.payer.key();
        let seeds=&[b"pool",payer_key.as_ref(), &[ctx.accounts.pool.bump]];
        let signer=&[&seeds[..]];
        
        if is_a_to_b {
            // User sends Token A
            token::transfer(
                CpiContext::new(
                    ctx.accounts.token_program.to_account_info(),
                    Transfer {
                        from: ctx.accounts.user_token_a.to_account_info(),
                        to: ctx.accounts.vault_a.to_account_info(),
                        authority: ctx.accounts.payer.to_account_info(),
                    },
                ),
                amount,
            )?;
            // Pool sends Token B
            token::transfer(
                CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info(),
                    Transfer {
                        from: ctx.accounts.vault_b.to_account_info(),
                        to: ctx.accounts.user_token_b.to_account_info(),
                        authority: ctx.accounts.pool.to_account_info(),
                    },
                    signer,
                ),
                amount_out,
            )?;
        } else {
            // User sends Token B
            token::transfer(
                CpiContext::new(
                    ctx.accounts.token_program.to_account_info(),
                    Transfer {
                        from: ctx.accounts.user_token_b.to_account_info(),
                        to: ctx.accounts.vault_b.to_account_info(),
                        authority: ctx.accounts.payer.to_account_info(),
                    },
                ),
                amount,
            )?;
            // Pool sends Token A
            token::transfer(
                CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info(),
                    Transfer {
                        from: ctx.accounts.vault_a.to_account_info(),
                        to: ctx.accounts.user_token_a.to_account_info(),
                        authority: ctx.accounts.pool.to_account_info(),
                    },
                    signer,
                ),
                amount_out,
            )?;
        }
        Ok(())
    }
}

#[account]
pub struct Pool{
    pub token_a_mint:Pubkey,
    pub token_b_mint:Pubkey,
    pub fees_percentage:u16,
    pub bump:u8
}

#[derive(Accounts)]
pub struct Initialize<'info>{
    #[account(mut)]
    pub payer:Signer<'info>,
    #[account(
        init,
        payer=payer,
        space=8+32+32+2+1,
        seeds=[b"pool",payer.key().as_ref()],
        bump
    )]
    pub pool:Account<'info,Pool>,
    pub token_a_mint:Account<'info,Mint>,
    pub token_b_mint:Account<'info,Mint>,
    #[account(
        init,
        payer=payer,
        token::mint=token_a_mint,
        token::authority=pool
    )]
    pub vault_a:Account<'info,TokenAccount>,
    #[account(
        init,
        payer=payer,
        token::mint=token_b_mint,
        token::authority=pool
    )]
    pub vault_b:Account<'info,TokenAccount>,
    #[account(
        init,
        payer=payer,
        mint::decimals=6,
        mint::authority=pool,
    )]
    pub lp_mint:Account<'info,Mint>,
    pub token_program:Program<'info,Token>,
    pub system_program:Program<'info,System>,
    pub rent:Sysvar<'info,Rent>,
}

#[derive(Accounts)]
pub struct Deposite<'info>{
    #[account(mut)]
    pub payer:Signer<'info>,
    #[account(
        mut,
        seeds=[b"pool",payer.key().as_ref()],
        bump=pool.bump,
    )]
    pub pool:Account<'info,Pool>,
    #[account(mut)]
    pub user_token_a:Account<'info,TokenAccount>,
    #[account(mut)]
    pub user_token_b:Account<'info,TokenAccount>,
    #[account(mut)]
    pub vault_a:Account<'info,TokenAccount>,
    #[account(mut)]
    pub vault_b:Account<'info,TokenAccount>,
    #[account(mut)]
    pub lp_mint:Account<'info,Mint>,
    #[account(mut)]
    pub user_token_lp:Account<'info,TokenAccount>,
    pub token_program:Program<'info,Token>,
}

#[derive(Accounts)]
pub struct RemoveLiquidity<'info>{
    #[account(mut)]
    pub payer:Signer<'info>,
    #[account(
        mut,
        seeds=[b"pool",payer.key().as_ref()],
        bump=pool.bump,
    )]
    pub pool:Account<'info,Pool>,
    #[account(mut)]
    pub user_token_a:Account<'info,TokenAccount>,
    #[account(mut)]
    pub user_token_b:Account<'info,TokenAccount>,
    #[account(mut)]
    pub vault_a:Account<'info,TokenAccount>,
    #[account(mut)]
    pub vault_b:Account<'info,TokenAccount>,
    #[account(mut)]
    pub lp_mint:Account<'info,Mint>,
    #[account(mut)]
    pub user_token_lp:Account<'info,TokenAccount>,
    pub token_program:Program<'info,Token>,
}

#[derive(Accounts)]
pub struct Swap<'info>{
    #[account(mut)]
    pub payer:Signer<'info>,
    #[account(
        mut,
        seeds=[b"pool",payer.key().as_ref()],
        bump=pool.bump,
    )]
    pub pool:Account<'info,Pool>,
    #[account(mut)]
    pub user_token_a:Account<'info,TokenAccount>,
    #[account(mut)]
    pub user_token_b:Account<'info,TokenAccount>,
    #[account(mut)]
    pub vault_a:Account<'info,TokenAccount>,
    #[account(mut)]
    pub vault_b:Account<'info,TokenAccount>,
    pub token_program:Program<'info,Token>,
}

#[error_code]
pub enum AMMErrors {
    #[msg("invalid amount")]
    InvalidAmount,
}

trait IntegerSqrt {
    fn integer_sqrt(&self) -> Self;
}

impl IntegerSqrt for u128 {
    fn integer_sqrt(&self) -> Self {
        if *self == 0 { return 0; }
        let mut x = *self;
        let mut y = (x + 1) / 2;
        while y < x {
            x = y;
            y = (x + *self / x) / 2;
        }
        x
    }
}
