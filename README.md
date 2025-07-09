# anchor-vault

A simple and secure vault implementation for Solana using the Anchor framework. This program allows users to create personal vaults for storing and managing SOL with built-in safety features.

## Features

- **Personal Vaults**: Each user gets their own isolated vault using Program Derived Addresses (PDAs)
- **Secure Operations**: Deposit, withdraw, and close vault operations with validation
- **Rent Exemption**: Automatic handling of Solana rent requirements
- **Minimum Deposits**: Enforces minimum deposit of 1000 lamports (0.000001 SOL)
- **Withdrawal Limits**: Maximum withdrawal of 1,000,000,000,000 lamports
- **Event Logging**: Emits events for all vault operations for tracking

## Project Structure

```
├── programs/
│   └── anchor-vault/
│       └── src/
│           └── lib.rs          # Main program logic
├── tests/
│   └── anchor-vault.ts         # Comprehensive test suite
├── migrations/
│   └── deploy.ts               # Deployment script
└── target/
    └── types/                  # Generated TypeScript types
```

## Setup

### Prerequisites

- Node.js 16+
- Rust 1.70+
- Solana CLI
- Anchor CLI

### Installation

1. Clone the repository
2. Install dependencies:
```bash
yarn install
```

3. Build the program:
```bash
anchor build
```

## Usage

### Initialize a Vault

```typescript
await program.methods
  .initialize()
  .accounts({
    user: wallet.publicKey,
  })
  .rpc();
```

### Deposit Funds

```typescript
const amount = new anchor.BN(1000000); // 0.001 SOL
await program.methods
  .deposit(amount)
  .accounts({
    user: wallet.publicKey,
  })
  .rpc();
```

### Withdraw Funds

```typescript
const amount = new anchor.BN(500000); // 0.0005 SOL
await program.methods
  .withdraw(amount)
  .accounts({
    user: wallet.publicKey,
  })
  .rpc();
```

### Close Vault

```typescript
await program.methods
  .close()
  .accounts({
    user: wallet.publicKey,
  })
  .rpc();
```

## Testing

Run the comprehensive test suite:

```bash
anchor test
```

The tests cover:
- Vault initialization
- Deposit operations (including minimum amount validation)
- Withdrawal operations (including rent exemption checks)
- Vault closure
- Edge cases and error conditions
- Multiple user scenarios

## Program Details

### Account Structure

- **VaultState**: Stores bump seeds for PDA derivation
- **Vault**: System account that holds the actual SOL funds

### PDA Seeds

- Vault State: `["state", user_pubkey]`
- Vault Account: `["vault", user_pubkey]`

### Error Codes

- `InsufficientDepositAmount`: Deposit below 1000 lamports
- `InvalidWithdrawAmount`: Withdrawal amount is zero
- `ExceedsMaxWithdrawal`: Withdrawal exceeds maximum limit
- `InsufficientFundsAfterWithdrawal`: Would break rent exemption

## Security Features

- PDAs ensure only the vault owner can access funds
- Rent exemption validation prevents account closure
- Input validation on all operations
- Proper CPI (Cross-Program Invocation) usage for transfers

## Development

### Build
```bash
anchor build
```

### Test
```bash
anchor test
```

### Deploy
```bash
anchor deploy
```

### Format Code
```bash
yarn lint:fix
```

## License

ISC
