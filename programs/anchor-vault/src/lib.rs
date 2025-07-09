#![allow(unexpected_cfgs)]
#![allow(deprecated)]
use anchor_lang::{
    prelude::*,
    system_program::{transfer, Transfer},
};

declare_id!("7HE7YJRihTBcn2Abk2kqGoT2i5o6wazR1wv8ursmxv9u");

// Program constants
const MIN_DEPOSIT_AMOUNT: u64 = 1000; // (0.000001 SOL)
const MAX_WITHDRAWAL_AMOUNT: u64 = 1_000_000_000_000;

#[program]
pub mod anchor_vault {
    use super::*;

    /**
     * @notice Initializes a new vault for the user
     * @dev Creates both vault state account and vault system account with proper PDAs
     * @param ctx Initialize context containing user, vault_state, vault, and system_program
     * @return Result<()> Success or error
     */
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Initializing vault for user: {}", ctx.accounts.user.key());
        ctx.accounts.initialize(&ctx.bumps)?;
        
        emit!(VaultInitialized {
            user: ctx.accounts.user.key(),
            vault: ctx.accounts.vault.key(),
            vault_state: ctx.accounts.vault_state.key(),
        });
        
        Ok(())
    }

    /**
     * @notice Deposits funds into the user's vault
     * @dev Transfers lamports from user to vault with validation
     * @param ctx Payment context
     * @param amount Amount to deposit in lamports
     * @return Result<()> Success or error
     */
    pub fn deposit(ctx: Context<Payment>, amount: u64) -> Result<()> {
        require!(amount >= MIN_DEPOSIT_AMOUNT, VaultError::InsufficientDepositAmount);
        
        msg!("Depositing {} lamports to vault: {}", amount, ctx.accounts.vault.key());
        ctx.accounts.deposit(amount)?;
        
        emit!(FundsDeposited {
            user: ctx.accounts.user.key(),
            vault: ctx.accounts.vault.key(),
            amount,
        });
        
        Ok(())
    }

    /**
     * @notice Withdraws funds from the user's vault
     * @dev Transfers lamports from vault to user with rent exemption check
     * @param ctx Payment context
     * @param amount Amount to withdraw in lamports
     * @return Result<()> Success or error
     */
    pub fn withdraw(ctx: Context<Payment>, amount: u64) -> Result<()> {
        require!(amount > 0, VaultError::InvalidWithdrawAmount);
        require!(amount <= MAX_WITHDRAWAL_AMOUNT, VaultError::ExceedsMaxWithdrawal);
        
        let vault_balance = ctx.accounts.vault.get_lamports();
        let rent_exempt = Rent::get()?.minimum_balance(ctx.accounts.vault.to_account_info().data_len());
        
        require!(
            vault_balance.saturating_sub(amount) >= rent_exempt,
            VaultError::InsufficientFundsAfterWithdrawal
        );
        
        msg!("Withdrawing {} lamports from vault: {}", amount, ctx.accounts.vault.key());
        ctx.accounts.withdraw(amount)?;
        
        emit!(FundsWithdrawn {
            user: ctx.accounts.user.key(),
            vault: ctx.accounts.vault.key(),
            amount,
        });
        
        Ok(())
    }

    /**
     * @notice Closes the vault and transfers all remaining funds to user
     * @dev Drains vault completely and closes the vault state account
     * @param ctx Close context
     * @return Result<()> Success or error
     */
    pub fn close(ctx: Context<Close>) -> Result<()> {
        let vault_balance = ctx.accounts.vault.get_lamports();
        
        msg!("Closing vault: {} with balance: {}", ctx.accounts.vault.key(), vault_balance);
        ctx.accounts.close()?;
        
        emit!(VaultClosed {
            user: ctx.accounts.user.key(),
            vault: ctx.accounts.vault.key(),
            final_balance: vault_balance,
        });
        
        Ok(())
    }
}

/**
 * @notice Account validation struct for vault initialization
 * @dev Creates PDA accounts for vault state and vault with proper seeds
 */
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        init,
        payer = user,
        space = VaultState::DISCRIMINATOR.len() + VaultState::INIT_SPACE,
        seeds = [VaultState::STATE_SEED, user.key().as_ref()],
        bump,
    )]
    pub vault_state: Account<'info, VaultState>,

    #[account(
        mut,
        seeds = [VaultState::VAULT_SEED, user.key().as_ref()],
        bump,
    )]
    pub vault: SystemAccount<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> Initialize<'info> {
    /**
     * @notice Initializes vault state and funds vault with rent-exempt amount
     * @dev Sets bump seeds and transfers minimum balance for rent exemption
     * @param bumps Bump seeds from account initialization
     * @return Result<()> Success or error
     */
    fn initialize(&mut self, bumps: &InitializeBumps) -> Result<()> {
        // Initialize vault state with bump seeds
        self.vault_state.set_inner(VaultState {
            state_bump: bumps.vault_state,
            vault_bump: bumps.vault,
        });

        // Calculate and transfer rent-exempt amount to vault
        let rent_exempt = Rent::get()?.minimum_balance(self.vault.to_account_info().data_len());

        let transfer_accounts = Transfer {
            from: self.user.to_account_info(),
            to: self.vault.to_account_info(),
        };

        let transfer_ctx = CpiContext::new(self.system_program.to_account_info(), transfer_accounts);

        transfer(transfer_ctx, rent_exempt)
    }
}

/**
 * @notice Account validation struct for deposit and withdrawal operations
 * @dev Validates vault ownership and account relationships
 */
#[derive(Accounts)]
pub struct Payment<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        seeds = [VaultState::STATE_SEED, user.key().as_ref()],
        bump = vault_state.state_bump
    )]
    pub vault_state: Account<'info, VaultState>,

    #[account(
        mut,
        seeds = [VaultState::VAULT_SEED, user.key().as_ref()],
        bump = vault_state.vault_bump
    )]
    pub vault: SystemAccount<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> Payment<'info> {
    /**
     * @notice Deposits funds from user to vault
     * @dev Transfers lamports using system program CPI
     * @param amount Amount to deposit in lamports
     * @return Result<()> Success or error
     */
    fn deposit(&mut self, amount: u64) -> Result<()> {
        let transfer_accounts = Transfer {
            from: self.user.to_account_info(),
            to: self.vault.to_account_info(),
        };

        let transfer_ctx = CpiContext::new(self.system_program.to_account_info(), transfer_accounts);

        transfer(transfer_ctx, amount)
    }

    /**
     * @notice Withdraws funds from vault to user
     * @dev Uses PDA signing to authorize transfer from vault
     * @param amount Amount to withdraw in lamports
     * @return Result<()> Success or error
     */
    fn withdraw(&mut self, amount: u64) -> Result<()> {
        let transfer_accounts = Transfer {
            from: self.vault.to_account_info(),
            to: self.user.to_account_info(),
        };

        // Create PDA seeds for vault signing
        let seeds = &[
            VaultState::VAULT_SEED,
            self.user.to_account_info().key.as_ref(),
            &[self.vault_state.vault_bump],
        ];
        let signer_seeds = &[&seeds[..]];

        let transfer_ctx = CpiContext::new_with_signer(
            self.system_program.to_account_info(),
            transfer_accounts,
            signer_seeds,
        );

        transfer(transfer_ctx, amount)?;

        // Verify vault maintains rent exemption after withdrawal
        let rent_exempt = Rent::get()?.minimum_balance(self.vault.to_account_info().data_len());
        require_gte!(self.vault.get_lamports(), rent_exempt);

        Ok(())
    }
}

/**
 * @notice Account validation struct for vault closure
 * @dev Closes vault state account and transfers remaining funds
 */
#[derive(Accounts)]
pub struct Close<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        close = user,
        seeds = [VaultState::STATE_SEED, user.key().as_ref()],
        bump = vault_state.state_bump
    )]
    pub vault_state: Account<'info, VaultState>,

    #[account(
        mut,
        seeds = [VaultState::VAULT_SEED, user.key().as_ref()],
        bump = vault_state.vault_bump
    )]
    pub vault: SystemAccount<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> Close<'info> {
    /**
     * @notice Closes vault and transfers all remaining funds to user
     * @dev Drains vault completely using PDA signing
     * @return Result<()> Success or error
     */
    fn close(&mut self) -> Result<()> {
        let transfer_accounts = Transfer {
            from: self.vault.to_account_info(),
            to: self.user.to_account_info(),
        };

        // Create PDA seeds for vault signing
        let seeds = &[
            VaultState::VAULT_SEED,
            self.user.to_account_info().key.as_ref(),
            &[self.vault_state.vault_bump],
        ];
        let signer_seeds = &[&seeds[..]];

        let transfer_ctx = CpiContext::new_with_signer(
            self.system_program.to_account_info(),
            transfer_accounts,
            signer_seeds,
        );

        transfer(transfer_ctx, self.vault.get_lamports())
    }
}

/**
 * @notice Vault state account data structure
 * @dev Stores bump seeds for PDA derivation
 */
#[account]
#[derive(InitSpace)]
pub struct VaultState {
    /// Bump seed for vault state PDA
    pub state_bump: u8,
    /// Bump seed for vault PDA
    pub vault_bump: u8,
}

impl VaultState {
    /// Seed constant for vault state PDA
    pub const STATE_SEED: &'static [u8] = b"state";
    /// Seed constant for vault PDA
    pub const VAULT_SEED: &'static [u8] = b"vault";
}

// Events for program activity tracking

/**
 * @notice Event emitted when a vault is initialized
 */
#[event]
pub struct VaultInitialized {
    pub user: Pubkey,
    pub vault: Pubkey,
    pub vault_state: Pubkey,
}

/**
 * @notice Event emitted when funds are deposited
 */
#[event]
pub struct FundsDeposited {
    pub user: Pubkey,
    pub vault: Pubkey,
    pub amount: u64,
}

/**
 * @notice Event emitted when funds are withdrawn
 */
#[event]
pub struct FundsWithdrawn {
    pub user: Pubkey,
    pub vault: Pubkey,
    pub amount: u64,
}

/**
 * @notice Event emitted when a vault is closed
 */
#[event]
pub struct VaultClosed {
    pub user: Pubkey,
    pub vault: Pubkey,
    pub final_balance: u64,
}

// Custom error definitions

/**
 * @notice Custom error codes for vault operations
 */
#[error_code]
pub enum VaultError {
    #[msg("Deposit amount must be at least 1000 lamports")]
    InsufficientDepositAmount,
    
    #[msg("Withdrawal amount must be greater than 0")]
    InvalidWithdrawAmount,
    
    #[msg("Withdrawal amount exceeds maximum allowed")]
    ExceedsMaxWithdrawal,
    
    #[msg("Insufficient funds in vault after withdrawal to maintain rent exemption")]
    InsufficientFundsAfterWithdrawal,
}
