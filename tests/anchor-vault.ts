import * as anchor from '@coral-xyz/anchor';
import { Program } from '@coral-xyz/anchor';
import { expect } from 'chai';
import { AnchorVault } from '../target/types/anchor_vault';

describe('anchor-vault', () => {
  // Configure the client to use the local cluster
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.anchorVault as Program<AnchorVault>;
  const provider = anchor.getProvider();
  const wallet = provider.wallet;
  
  // Test constants
  const MIN_DEPOSIT_AMOUNT = new anchor.BN(1000);
  const STANDARD_DEPOSIT = new anchor.BN(anchor.web3.LAMPORTS_PER_SOL);
  
  // PDA derivation helpers
  const deriveVaultState = (userKey: anchor.web3.PublicKey) => {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from('state'), userKey.toBuffer()],
      program.programId
    );
  };

  const deriveVault = (seedKey: anchor.web3.PublicKey) => {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from('vault'), seedKey.toBuffer()],
      program.programId
    );
  };

  let vaultState: anchor.web3.PublicKey;
  let vault: anchor.web3.PublicKey;

  before(async () => {
    // Derive PDAs for the main user
    [vaultState] = deriveVaultState(wallet.publicKey);
    // For initialization, vault is derived from user key
    [vault] = deriveVault(wallet.publicKey);
  });

  describe('Initialization', () => {
    it('should initialize vault successfully', async () => {
      const tx = await program.methods
        .initialize()
        .accounts({
          user: wallet.publicKey,
        })
        .rpc();

      console.log('Initialize transaction signature:', tx);

      // Verify vault state account was created
      const vaultStateAccount = await program.account.vaultState.fetch(vaultState);
      expect(vaultStateAccount).to.not.be.null;

      // Check vault account was funded with rent-exempt amount
      const vaultAccountInfo = await provider.connection.getAccountInfo(vault);
      expect(vaultAccountInfo).to.not.be.null;
      expect(vaultAccountInfo!.lamports).to.be.greaterThan(0);
    });
  });

  describe('Deposits', () => {
    it('should deposit funds successfully', async () => {
      const initialBalance = await provider.connection.getBalance(vault);
      
      const tx = await program.methods
        .deposit(STANDARD_DEPOSIT)
        .accounts({
          user: wallet.publicKey,
        })
        .rpc();

      console.log('Deposit transaction signature:', tx);

      // Verify balance increased
      const finalBalance = await provider.connection.getBalance(vault);
      expect(finalBalance - initialBalance).to.equal(STANDARD_DEPOSIT.toNumber());
    });

    it('should deposit minimum amount successfully', async () => {
      const initialBalance = await provider.connection.getBalance(vault);
      
      const tx = await program.methods
        .deposit(MIN_DEPOSIT_AMOUNT)
        .accounts({
          user: wallet.publicKey,
        })
        .rpc();

      console.log('Minimum deposit transaction signature:', tx);

      // Verify balance increased
      const finalBalance = await provider.connection.getBalance(vault);
      expect(finalBalance - initialBalance).to.equal(MIN_DEPOSIT_AMOUNT.toNumber());
    });

    it('should fail to deposit amount below minimum', async () => {
      const tooSmallAmount = new anchor.BN(500); // Less than 1000 lamports

      try {
        await program.methods
          .deposit(tooSmallAmount)
          .accounts({
            user: wallet.publicKey,
          })
          .rpc();
        
        expect.fail('Should have failed with insufficient deposit amount');
      } catch (error) {
        expect(error.message).to.include('InsufficientDepositAmount');
      }
    });
  });

  describe('Withdrawals', () => {
    it('should withdraw funds successfully', async () => {
      const withdrawAmount = new anchor.BN(anchor.web3.LAMPORTS_PER_SOL / 2);
      const initialVaultBalance = await provider.connection.getBalance(vault);
      
      const tx = await program.methods
        .withdraw(withdrawAmount)
        .accounts({
          user: wallet.publicKey,
        })
        .rpc();

      console.log('Withdraw transaction signature:', tx);

      // Verify vault balance decreased
      const finalVaultBalance = await provider.connection.getBalance(vault);
      expect(initialVaultBalance - finalVaultBalance).to.equal(withdrawAmount.toNumber());
    });

    it('should fail to withdraw zero amount', async () => {
      try {
        await program.methods
          .withdraw(new anchor.BN(0))
          .accounts({
            user: wallet.publicKey,
          })
          .rpc();
        
        expect.fail('Should have failed with zero withdrawal amount');
      } catch (error) {
        expect(error.message).to.include('InvalidWithdrawAmount');
      }
    });

    it('should fail to withdraw amount that would break rent exemption', async () => {
      const vaultBalance = await provider.connection.getBalance(vault);
      const rentExempt = await provider.connection.getMinimumBalanceForRentExemption(0);
      
      // Try to withdraw almost all funds, leaving less than rent exempt
      const excessiveAmount = new anchor.BN(vaultBalance - rentExempt + 1);

      try {
        await program.methods
          .withdraw(excessiveAmount)
          .accounts({
            user: wallet.publicKey,
          })
          .rpc();
        
        expect.fail('Should have failed with insufficient funds after withdrawal');
      } catch (error) {
        expect(error.message).to.include('InsufficientFundsAfterWithdrawal');
      }
    });
  });

  describe('Vault Closure', () => {
    it('should close vault and transfer all funds', async () => {
      const initialVaultBalance = await provider.connection.getBalance(vault);
      
      const tx = await program.methods
        .close()
        .accounts({
          user: wallet.publicKey,
        })
        .rpc();

      console.log('Close transaction signature:', tx);

      // Verify vault state account was closed
      try {
        await program.account.vaultState.fetch(vaultState);
        expect.fail('Vault state account should have been closed');
      } catch (error) {
        expect(error.message).to.include('Account does not exist');
      }

      // Verify vault was drained
      const finalVaultBalance = await provider.connection.getBalance(vault);
      expect(finalVaultBalance).to.equal(0);
    });
  });

  describe('Edge Cases', () => {
    let newUser: anchor.web3.Keypair;
    let newUserVaultState: anchor.web3.PublicKey;
    let newUserVault: anchor.web3.PublicKey;

    before(async () => {
      // Create a new user for testing
      newUser = anchor.web3.Keypair.generate();
      
      // Airdrop some SOL to the new user
      const signature = await provider.connection.requestAirdrop(
        newUser.publicKey,
        2 * anchor.web3.LAMPORTS_PER_SOL
      );
      await provider.connection.confirmTransaction(signature);

      // Derive PDAs for new user
      [newUserVaultState] = deriveVaultState(newUser.publicKey);
      [newUserVault] = deriveVault(newUser.publicKey);
    });

    it('should fail operations on uninitialized vault', async () => {
      try {
        await program.methods
          .deposit(STANDARD_DEPOSIT)
          .accounts({
            user: newUser.publicKey,
          })
          .signers([newUser])
          .rpc();
        
        expect.fail('Should have failed on uninitialized vault');
      } catch (error) {
        // Error occurs during account validation since vault_state doesn't exist
        expect(error.message).to.include('vault_state');
      }
    });

    it('should handle multiple user vaults independently', async () => {
      // Initialize vault for new user
      const tx = await program.methods
        .initialize()
        .accounts({
          user: newUser.publicKey,
        })
        .signers([newUser])
        .rpc();

      console.log('New user initialize transaction signature:', tx);

      // Verify new user's vault is independent
      const newUserVaultStateAccount = await program.account.vaultState.fetch(newUserVaultState);
      expect(newUserVaultStateAccount).to.not.be.null;
      
      // Clean up - close the new user's vault
      await program.methods
        .close()
        .accounts({
          user: newUser.publicKey,
        })
        .signers([newUser])
        .rpc();
    });
  });
});