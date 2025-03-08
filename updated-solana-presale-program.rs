use anchor_lang::prelude::*;
use anchor_spl::token_2022::{self, Mint, Token2022, TokenAccount, Transfer};
use solana_program::pubkey::Pubkey;

declare_id!("Presaf1E5aLeSoLanaT0kenSa1eePr0graM111111");

#[program]
pub mod solana_presale {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        rate: u64,
        presale_start: i64,
        presale_end: i64,
        min_purchase: u64,
        max_purchase: u64,
    ) -> Result<()> {
        let presale = &mut ctx.accounts.presale;
        presale.admin = ctx.accounts.admin.key();
        presale.token_mint = ctx.accounts.token_mint.key();
        presale.treasury = ctx.accounts.treasury.key();
        presale.presale_token_account = ctx.accounts.presale_token_account.key();
        presale.rate = rate; // How many tokens per lamport
        presale.presale_start = presale_start;
        presale.presale_end = presale_end;
        presale.min_purchase = min_purchase;
        presale.max_purchase = max_purchase;
        presale.total_sold = 0;
        presale.is_active = true;
        
        Ok(())
    }

    pub fn buy_tokens(ctx: Context<BuyTokens>, amount_sol: u64) -> Result<()> {
        let presale = &mut ctx.accounts.presale;
        let clock = Clock::get()?;
        
        // Check if presale is active
        require!(presale.is_active, PresaleError::PresaleNotActive);
        
        // Check if presale has started
        require!(
            clock.unix_timestamp >= presale.presale_start,
            PresaleError::PresaleNotStarted
        );
        
        // Check if presale has ended
        require!(
            clock.unix_timestamp <= presale.presale_end,
            PresaleError::PresaleEnded
        );
        
        // Check if purchase amount is within limits
        require!(
            amount_sol >= presale.min_purchase,
            PresaleError::BelowMinimumPurchase
        );
        require!(
            amount_sol <= presale.max_purchase,
            PresaleError::AboveMaximumPurchase
        );
        
        // Calculate tokens to purchase using safe math
        let tokens_to_purchase = amount_sol
            .checked_mul(presale.rate)
            .ok_or(PresaleError::CalculationError)?;
        
        // Update total sold using safe math
        presale.total_sold = presale.total_sold
            .checked_add(tokens_to_purchase)
            .ok_or(PresaleError::CalculationError)?;
        
        // Transfer SOL from buyer to treasury
        let transfer_instruction = anchor_lang::solana_program::system_instruction::transfer(
            &ctx.accounts.buyer.key(),
            &ctx.accounts.treasury.key(),
            amount_sol,
        );
        
        anchor_lang::solana_program::program::invoke(
            &transfer_instruction,
            &[
                ctx.accounts.buyer.to_account_info(),
                ctx.accounts.treasury.to_account_info(),
            ],
        )?;
        
        // Transfer tokens from presale token account to buyer token account using Token-2022
        let transfer_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.presale_token_account.to_account_info(),
                to: ctx.accounts.buyer_token_account.to_account_info(),
                authority: ctx.accounts.presale.to_account_info(),
            },
        );
        
        token_2022::transfer(
            transfer_ctx.with_signer(&[&[b"presale", &[*ctx.bumps.get("presale").unwrap()]]]),
            tokens_to_purchase,
        )?;
        
        Ok(())
    }
    
    pub fn toggle_presale(ctx: Context<TogglePresale>, is_active: bool) -> Result<()> {
        let presale = &mut ctx.accounts.presale;
        
        // Only admin can toggle the presale
        require!(
            presale.admin == ctx.accounts.admin.key(),
            PresaleError::Unauthorized
        );
        
        presale.is_active = is_active;
        
        Ok(())
    }
    
    pub fn withdraw_unsold_tokens(ctx: Context<WithdrawUnsold>) -> Result<()> {
        let presale = &ctx.accounts.presale;
        
        // Only admin can withdraw unsold tokens
        require!(
            presale.admin == ctx.accounts.admin.key(),
            PresaleError::Unauthorized
        );
        
        // Check if presale has ended
        let clock = Clock::get()?;
        require!(
            clock.unix_timestamp > presale.presale_end,
            PresaleError::PresaleNotEnded
        );
        
        // Get the balance of tokens in the presale token account
        let token_balance = ctx.accounts.presale_token_account.amount;
        
        // Transfer all remaining tokens from presale token account to admin token account using Token-2022
        let transfer_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.presale_token_account.to_account_info(),
                to: ctx.accounts.admin_token_account.to_account_info(),
                authority: ctx.accounts.presale.to_account_info(),
            },
        );
        
        token_2022::transfer(
            transfer_ctx.with_signer(&[&[b"presale", &[*ctx.bumps.get("presale").unwrap()]]]),
            token_balance,
        )?;
        
        Ok(())
    }
    
    pub fn withdraw_sol(ctx: Context<WithdrawSol>, amount: Option<u64>) -> Result<()> {
        let presale = &ctx.accounts.presale;
        
        // Only admin can withdraw SOL
        require!(
            presale.admin == ctx.accounts.admin.key(),
            PresaleError::Unauthorized
        );
        
        // Get treasury account
        let treasury = &ctx.accounts.treasury;
        let treasury_lamports = treasury.lamports();
        
        // Determine amount to withdraw (all if None is provided)
        let withdraw_amount = match amount {
            Some(amt) => {
                // Check if requested amount is available
                require!(
                    amt <= treasury_lamports,
                    PresaleError::InsufficientBalance
                );
                amt
            },
            None => {
                // Withdraw all SOL
                treasury_lamports
            }
        };
        
        // Safely calculate new balances using checked arithmetic
        let new_treasury_balance = treasury_lamports
            .checked_sub(withdraw_amount)
            .ok_or(PresaleError::CalculationError)?;
            
        let new_admin_balance = ctx.accounts.admin.lamports()
            .checked_add(withdraw_amount)
            .ok_or(PresaleError::CalculationError)?;
        
        // Transfer SOL from treasury to admin using safe operations
        **treasury.try_borrow_mut_lamports()? = new_treasury_balance;
        **ctx.accounts.admin.try_borrow_mut_lamports()? = new_admin_balance;
        
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = admin,
        space = 8 + 32 + 32 + 32 + 32 + 8 + 8 + 8 + 8 + 8 + 8 + 1,
        seeds = [b"presale"],
        bump
    )]
    pub presale: Account<'info, Presale>,
    
    #[account(mut)]
    pub admin: Signer<'info>,
    
    pub token_mint: Account<'info, Mint>,
    
    /// CHECK: This is safe because we only store the address
    #[account(mut)]
    pub treasury: AccountInfo<'info>,
    
    #[account(
        mut,
        constraint = presale_token_account.mint == token_mint.key(),
        constraint = presale_token_account.owner == presale.key(),
    )]
    pub presale_token_account: Account<'info, TokenAccount>,
    
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token2022>, // Changed from Token to Token2022
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct BuyTokens<'info> {
    #[account(
        mut,
        seeds = [b"presale"],
        bump,
    )]
    pub presale: Account<'info, Presale>,
    
    #[account(mut)]
    pub buyer: Signer<'info>,
    
    /// CHECK: This account is validated in the instruction
    #[account(mut, address = presale.treasury)]
    pub treasury: AccountInfo<'info>,
    
    #[account(
        mut,
        address = presale.presale_token_account,
    )]
    pub presale_token_account: Account<'info, TokenAccount>,
    
    #[account(
        mut,
        constraint = buyer_token_account.owner == buyer.key(),
        constraint = buyer_token_account.mint == token_mint.key(),
    )]
    pub buyer_token_account: Account<'info, TokenAccount>,
    
    #[account(address = presale.token_mint)]
    pub token_mint: Account<'info, Mint>,
    
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token2022>, // Changed from Token to Token2022
}

#[derive(Accounts)]
pub struct TogglePresale<'info> {
    #[account(
        mut,
        seeds = [b"presale"],
        bump,
    )]
    pub presale: Account<'info, Presale>,
    
    #[account(mut)]
    pub admin: Signer<'info>,
}

#[derive(Accounts)]
pub struct WithdrawUnsold<'info> {
    #[account(
        mut,
        seeds = [b"presale"],
        bump,
    )]
    pub presale: Account<'info, Presale>,
    
    #[account(mut)]
    pub admin: Signer<'info>,
    
    #[account(
        mut,
        address = presale.presale_token_account,
    )]
    pub presale_token_account: Account<'info, TokenAccount>,
    
    #[account(
        mut,
        constraint = admin_token_account.owner == admin.key(),
        constraint = admin_token_account.mint == token_mint.key(),
    )]
    pub admin_token_account: Account<'info, TokenAccount>,
    
    #[account(address = presale.token_mint)]
    pub token_mint: Account<'info, Mint>,
    
    pub token_program: Program<'info, Token2022>, // Changed from Token to Token2022
}

#[derive(Accounts)]
pub struct WithdrawSol<'info> {
    #[account(
        seeds = [b"presale"],
        bump,
    )]
    pub presale: Account<'info, Presale>,
    
    #[account(mut)]
    pub admin: Signer<'info>,
    
    /// CHECK: This account is validated in the instruction
    #[account(mut, address = presale.treasury)]
    pub treasury: AccountInfo<'info>,
}

#[account]
pub struct Presale {
    pub admin: Pubkey,
    pub token_mint: Pubkey,
    pub treasury: Pubkey,
    pub presale_token_account: Pubkey,
    pub rate: u64,
    pub presale_start: i64,
    pub presale_end: i64,
    pub min_purchase: u64,
    pub max_purchase: u64,
    pub total_sold: u64,
    pub is_active: bool,
}

#[error_code]
pub enum PresaleError {
    #[msg("Presale is not active")]
    PresaleNotActive,
    #[msg("Presale has not started yet")]
    PresaleNotStarted,
    #[msg("Presale has already ended")]
    PresaleEnded,
    #[msg("Presale has not ended yet")]
    PresaleNotEnded,
    #[msg("Purchase amount is below minimum")]
    BelowMinimumPurchase,
    #[msg("Purchase amount is above maximum")]
    AboveMaximumPurchase,
    #[msg("Calculation error")]
    CalculationError,
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("Insufficient balance")]
    InsufficientBalance,
}
