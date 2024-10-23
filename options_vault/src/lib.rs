use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use anchor_lang::solana_program::clock;

// Declare the program ID
declare_id!("Bc8T4czW6Km9tn67f7BtdV3nwkzyoScCdQpcZ9z8drqu");

#[program]
pub mod options_vault {
    use super::*;

    // Function to initialize the vault
    pub fn initialize_vault(ctx: Context<InitializeVault>, bump: u8) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        vault.bump = bump;
        vault.total_deposits = 0;
        vault.reward_rate = 10; // Example rate for boosted rewards
        vault.authority = *ctx.accounts.authority.key; // Admin authority
        vault.paused = false; // Initialize paused state as false
        vault.total_profit = 0;
        vault.total_trades = 0;
        vault.last_strategy_execution = 0; // No strategy executed at initialization
        Ok(())
    }

    // Function to deposit assets into the vault
    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        let user = &mut ctx.accounts.user;

        // Ensure vault is not paused
        if vault.paused {
            return Err(ErrorCode::VaultPaused.into());
        }

        // Transfer the user's tokens to the vault's token account
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.user_token_account.to_account_info(),
                    to: ctx.accounts.vault_token_account.to_account_info(),
                    authority: user.to_account_info(),
                },
            ),
            amount,
        )?;

        // Update total deposits in the vault
        vault.total_deposits += amount;

        // Emit deposit event
        emit!(DepositEvent {
            user: *user.key,
            amount,
        });

        Ok(())
    }

    // Function to withdraw assets from the vault with a fee
    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        let fee_percentage = 5; // Example: 5% fee on withdrawals

        // Reentrancy protection and validate withdrawal amount
        if vault.total_deposits < amount {
            return Err(ErrorCode::InsufficientFunds.into());
        }

        // Calculate withdrawal fee and net amount after fee
        let fee = amount * fee_percentage / 100;
        let amount_after_fee = amount - fee;

        // Transfer tokens from the vault to the user's account after fee
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.vault_token_account.to_account_info(),
                    to: ctx.accounts.user_token_account.to_account_info(),
                    authority: ctx.accounts.vault_authority.to_account_info(),
                },
            ),
            amount_after_fee,
        )?;

        // Transfer fee to the admin's fee vault
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.vault_token_account.to_account_info(),
                    to: ctx.accounts.fee_vault_token_account.to_account_info(),
                    authority: ctx.accounts.vault_authority.to_account_info(),
                },
            ),
            fee,
        )?;

        // Update the vault's deposit balance
        vault.total_deposits -= amount;

        // Emit withdrawal event
        emit!(WithdrawEvent {
            user: *ctx.accounts.user.key,
            amount: amount_after_fee,
            fee,
        });

        Ok(())
    }

    // Emergency withdrawal function without fees
    pub fn emergency_withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        let vault = &mut ctx.accounts.vault;

        // Ensure vault has enough balance
        if vault.total_deposits < amount {
            return Err(ErrorCode::InsufficientFunds.into());
        }

        // Transfer full amount without fee
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.vault_token_account.to_account_info(),
                    to: ctx.accounts.user_token_account.to_account_info(),
                    authority: ctx.accounts.vault_authority.to_account_info(),
                },
            ),
            amount,
        )?;

        // Update the vault's deposit balance
        vault.total_deposits -= amount;

        Ok(())
    }

    // Function to execute the options strategy based on market conditions with frequency control
    pub fn execute_strategy(ctx: Context<ExecuteStrategy>, market_price: u64) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        let clock = clock::Clock::get()?;
        let min_time_between_strategies = 3600; // 1 hour (3600 seconds)

        // Ensure the last strategy execution was more than 1 hour ago
        if clock.unix_timestamp - vault.last_strategy_execution < min_time_between_strategies {
            return Err(ErrorCode::StrategyExecutionTooSoon.into());
        }

        // Simulate options strategy based on market price
        let profit_or_loss: i64;
        if market_price > vault.price_threshold {
            msg!("Executing covered call strategy as market price is high.");
            profit_or_loss = 1000;  // Example profit
        } else {
            msg!("Executing cash-secured put strategy as market price is low.");
            profit_or_loss = -500;  // Example loss
        }

        // Track performance
        vault.total_profit += profit_or_loss;
        vault.total_trades += 1;
        vault.last_strategy_execution = clock.unix_timestamp;

        // Emit strategy execution event
        emit!(StrategyExecutedEvent {
            strategy: "Covered Call or Put".to_string(),
            market_price,
            profit_or_loss,
            total_trades: vault.total_trades,
        });

        Ok(())
    }


    // Function to claim and auto-compound staking rewards
    pub fn claim_rewards(ctx: Context<ClaimRewards>) -> Result<()> {
        let clock = clock::Clock::get()?;
         let vault = &mut ctx.accounts.vault;  // Borrow `vault` mutably
         let user = &mut ctx.accounts.user;    // Also borrow `user` mutably

    // Calculate time since the user last claimed rewards
    let duration = clock.unix_timestamp - user.last_staked_timestamp;

    // Boosted rewards calculation based on staking duration
    let reward_multiplier = if duration >= 86400 * 30 { // Example: 30 days
        2  // Double the rewards after 30 days
    } else {
        1
    };

    let boosted_rewards = reward_multiplier * vault.reward_rate * duration as u64;
    user.reward_balance += boosted_rewards;

    // Auto-compound: Add rewards to user's deposit in the vault
    vault.total_deposits += user.reward_balance;  // Now mutably borrowing `vault`
    user.reward_balance = 0;  // Reset rewards after compounding

    // Update last staked timestamp
    user.last_staked_timestamp = clock.unix_timestamp;

    Ok(())
}
    // Function to allow borrowing for leverage with a cap
    pub fn borrow(ctx: Context<Borrow>, borrow_amount: u64) -> Result<()> {
        const MAX_LEVERAGE: u64 = 3; // Example leverage cap (3x)
        let vault = &ctx.accounts.vault;

        // Ensure user is not borrowing more than 3x their deposit
        if borrow_amount > vault.total_deposits * MAX_LEVERAGE {
            return Err(ErrorCode::ExcessiveLeverage.into());
        }

        msg!("User borrowing {} tokens using collateral.", borrow_amount);

        // Borrow logic would involve interaction with lending protocols like Solend/Port
        Ok(())
    }

    // Admin function to update the reward rate
    pub fn update_reward_rate(ctx: Context<AdminAction>, new_rate: u64) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        vault.reward_rate = new_rate;
        Ok(())
    }

    // Admin function to update the strategy threshold
    pub fn update_strategy_threshold(ctx: Context<AdminAction>, new_threshold: u64) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        vault.price_threshold = new_threshold;
        Ok(())
    }

    // Admin function to pause the vault
    pub fn pause_vault(ctx: Context<AdminAction>) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        vault.paused = true;
        Ok(())
    }

    // Admin function to unpause the vault
    pub fn unpause_vault(ctx: Context<AdminAction>) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        vault.paused = false;
        Ok(())
    }
}

/////////////////////////////////////////
// ACCOUNTS AND DATA STRUCTURES
/////////////////////////////////////////

#[account]
pub struct Vault {
    pub bump: u8,
    pub total_deposits: u64,
    pub reward_rate: u64,         // Reward rate for staking
    pub price_threshold: u64,     // Market price threshold for strategy execution
    pub authority: Pubkey,        // Admin authority
    pub paused: bool,             // Pause status
    pub total_profit: i64,        // Total profit from strategies
    pub total_trades: u64,        // Number of strategy executions
    pub last_strategy_execution: i64, // Timestamp of last strategy execution
}

#[account]
pub struct User {
    pub reward_balance: u64,       // Accumulated rewards for the user
    pub last_staked_timestamp: i64, // Timestamp of last staking activity
}

/////////////////////////////////////////
// CONTEXT STRUCTS
/////////////////////////////////////////

#[derive(Accounts)]
pub struct InitializeVault<'info> {
    #[account(init, payer = user, space = 8 + 8 + 8 + 32)]
    pub vault: Account<'info, Vault>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub vault: Account<'info, Vault>,
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub vault_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub vault: Account<'info, Vault>,
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub vault_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub fee_vault_token_account: Account<'info, TokenAccount>, // Admin fee vault
    pub vault_authority: AccountInfo<'info>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct ExecuteStrategy<'info> {
    #[account(mut)]
    pub vault: Account<'info, Vault>,
    pub user: Signer<'info>,
}

#[derive(Accounts)]
pub struct ClaimRewards<'info> {
    #[account(mut)]
    pub user: Account<'info, User>,
    #[account(mut)]
    pub vault: Account<'info, Vault>,
}

#[derive(Accounts)]
pub struct Borrow<'info> {
    #[account(mut)]
    pub vault: Account<'info, Vault>,
    pub user: Signer<'info>,
    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct AdminAction<'info> {
    #[account(mut, has_one = authority)]
    pub vault: Account<'info, Vault>,
    pub authority: Signer<'info>,
}

/////////////////////////////////////////
// EVENTS
/////////////////////////////////////////

#[event]
pub struct DepositEvent {
    pub user: Pubkey,
    pub amount: u64,
}

#[event]
pub struct WithdrawEvent {
    pub user: Pubkey,
    pub amount: u64,
    pub fee: u64, // Fee amount
}

#[event]
pub struct StrategyExecutedEvent {
    pub strategy: String,
    pub market_price: u64,
    pub profit_or_loss: i64,  // Profit or loss
    pub total_trades: u64,    // Total number of trades executed
}

/////////////////////////////////////////
// ERROR HANDLING
/////////////////////////////////////////

#[error_code]
pub enum ErrorCode {
    #[msg("Insufficient funds in the vault for this withdrawal.")]
    InsufficientFunds,
    #[msg("Unauthorized action.")]
    Unauthorized,
    #[msg("The vault is paused.")]
    VaultPaused,
    #[msg("The requested leverage exceeds the maximum allowed.")]
    ExcessiveLeverage,
    #[msg("Strategy execution too soon. Please wait.")]
    StrategyExecutionTooSoon,
}
