# Solana Token-2022 Presale Program

This program allows for conducting a presale of SPL Token-2022 tokens on the Solana blockchain. Users can purchase tokens with SOL at a fixed rate during a specified time window.

## Features

- Support for SPL Token-2022 standard tokens
- Configurable presale parameters (rate, start/end times, purchase limits)
- Admin controls to activate/deactivate the presale
- Functions to withdraw unsold tokens and collected SOL
- Time-based constraints for token purchases

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) 1.60 or later
- [Solana CLI](https://docs.solana.com/cli/install-solana-cli-tools) v1.10.0 or later
- [Anchor](https://project-serum.github.io/anchor/getting-started/installation.html) v0.24.0 or later
- [Node.js](https://nodejs.org) v16 or later

## Project Setup

1. Clone the repository:

```bash
git clone https://github.com/yourusername/solana-token2022-presale.git
cd solana-token2022-presale
```

2. Install dependencies:

```bash
npm install
```

3. Update your Anchor.toml and ensure the program ID is set correctly:

```toml
[programs.localnet]
solana_presale = "Presaf1E5aLeSoLanaT0kenSa1eePr0graM111111"

[programs.devnet]
solana_presale = "Presaf1E5aLeSoLanaT0kenSa1eePr0graM111111"

[programs.mainnet]
solana_presale = "Presaf1E5aLeSoLanaT0kenSa1eePr0graM111111"
```

## Building the Program

Build the program with Anchor:

```bash
anchor build
```

After building, get the program ID:

```bash
solana address -k target/deploy/solana_presale-keypair.json
```

Update the program ID in `lib.rs` and `Anchor.toml` if needed.

## Deploying to Devnet

1. Switch to Solana devnet:

```bash
solana config set --url devnet
```

2. Create a keypair for deployment if you don't already have one:

```bash
solana-keygen new -o deploy-keypair.json
```

3. Fund your deployment wallet:

```bash
solana airdrop 2 $(solana address -k deploy-keypair.json)
```

4. Deploy your program:

```bash
anchor deploy --provider.wallet deploy-keypair.json
```

## Running Tests

Run the tests against a local validator:

```bash
anchor test
```

To test on devnet:

```bash
anchor test --provider.cluster devnet
```

## Setting Up a Token-2022 Presale

1. Create a new SPL Token-2022 mint:

```javascript
const { createMint2022 } = require('@solana/spl-token-2022');
const connection = new web3.Connection(web3.clusterApiUrl('devnet'));
const adminKeypair = web3.Keypair.fromSecretKey(/* your admin secret key */);

const tokenMint = await createMint2022(
  connection,
  adminKeypair,
  adminKeypair.publicKey, // mint authority
  null, // freeze authority
  9, // 9 decimals
  undefined,
  undefined,
  TOKEN_2022_PROGRAM_ID
);

console.log(`Created token mint: ${tokenMint.toString()}`);
```

2. Find the presale PDA:

```javascript
const [presalePDA, presaleBump] = await web3.PublicKey.findProgramAddress(
  [Buffer.from('presale')],
  new web3.PublicKey('Presaf1E5aLeSoLanaT0kenSa1eePr0graM111111')
);

console.log(`Presale PDA: ${presalePDA.toString()}`);
```

3. Create a token account for the presale:

```javascript
const { createAccount: createAccount2022 } = require('@solana/spl-token-2022');

const presaleTokenAccount = await createAccount2022(
  connection,
  adminKeypair, // payer
  tokenMint,
  presalePDA,
  undefined,
  undefined,
  TOKEN_2022_PROGRAM_ID
);

console.log(`Presale token account: ${presaleTokenAccount.toString()}`);
```

4. Mint tokens to the presale account:

```javascript
const { mintTo: mintTo2022 } = require('@solana/spl-token-2022');

// Mint 1 million tokens (adjust based on token decimals)
await mintTo2022(
  connection,
  adminKeypair,
  tokenMint,
  presaleTokenAccount,
  adminKeypair.publicKey,
  1_000_000 * 10**9, // 1 million tokens with 9 decimals
  [],
  undefined,
  TOKEN_2022_PROGRAM_ID
);

console.log('Tokens minted to presale account');
```

5. Initialize the presale:

```javascript
const { BN } = require('@project-serum/anchor');
const { SystemProgram, SYSVAR_RENT_PUBKEY } = web3;

// Create a treasury wallet
const treasuryKeypair = web3.Keypair.generate();

// Presale parameters
const rate = new BN(1000); // 1000 tokens per SOL
const presaleStart = new BN(Math.floor(Date.now() / 1000)); // now
const presaleEnd = new BN(Math.floor(Date.now() / 1000) + 86400 * 7); // 7 days from now
const minPurchase = new BN(web3.LAMPORTS_PER_SOL / 10); // 0.1 SOL
const maxPurchase = new BN(web3.LAMPORTS_PER_SOL * 10); // 10 SOL

await program.methods.initialize(
  rate,
  presaleStart,
  presaleEnd,
  minPurchase,
  maxPurchase
)
.accounts({
  presale: presalePDA,
  admin: adminKeypair.publicKey,
  tokenMint: tokenMint,
  treasury: treasuryKeypair.publicKey,
  presaleTokenAccount: presaleTokenAccount,
  systemProgram: SystemProgram.programId,
  tokenProgram: TOKEN_2022_PROGRAM_ID,
  rent: web3.SYSVAR_RENT_PUBKEY
})
.signers([adminKeypair])
.rpc();

console.log('Presale initialized');
```

## Buying Tokens

For users to purchase tokens, they need to:

1. Create a Token-2022 account for receiving tokens:

```javascript
const buyerKeypair = web3.Keypair.fromSecretKey(/* buyer's secret key */);
const buyerTokenAccount = await createAccount2022(
  connection,
  buyerKeypair,
  tokenMint,
  buyerKeypair.publicKey,
  undefined,
  undefined,
  TOKEN_2022_PROGRAM_ID
);
```

2. Call the buy_tokens instruction:

```javascript
const purchaseAmount = web3.LAMPORTS_PER_SOL; // 1 SOL

await program.methods.buyTokens(new BN(purchaseAmount))
.accounts({
  presale: presalePDA,
  buyer: buyerKeypair.publicKey,
  treasury: treasuryKeypair.publicKey,
  presaleTokenAccount: presaleTokenAccount,
  buyerTokenAccount: buyerTokenAccount,
  tokenMint: tokenMint,
  systemProgram: SystemProgram.programId,
  tokenProgram: TOKEN_2022_PROGRAM_ID
})
.signers([buyerKeypair])
.rpc();

console.log('Tokens purchased successfully');
```

## Admin Operations

### Toggle Presale Status

The admin can activate or deactivate the presale:

```javascript
// Deactivate presale
await program.methods.togglePresale(false)
.accounts({
  presale: presalePDA,
  admin: adminKeypair.publicKey
})
.signers([adminKeypair])
.rpc();

console.log('Presale deactivated');

// Activate presale
await program.methods.togglePresale(true)
.accounts({
  presale: presalePDA,
  admin: adminKeypair.publicKey
})
.signers([adminKeypair])
.rpc();

console.log('Presale activated');
```

### Withdraw SOL from Treasury

The admin can withdraw SOL collected from token sales:

```javascript
// Withdraw specific amount
const withdrawAmount = new BN(web3.LAMPORTS_PER_SOL); // 1 SOL
await program.methods.withdrawSol(withdrawAmount)
.accounts({
  presale: presalePDA,
  admin: adminKeypair.publicKey,
  treasury: treasuryKeypair.publicKey
})
.signers([adminKeypair])
.rpc();

console.log(`${withdrawAmount} lamports withdrawn`);

// Withdraw all SOL
await program.methods.withdrawSol(null)
.accounts({
  presale: presalePDA,
  admin: adminKeypair.publicKey,
  treasury: treasuryKeypair.publicKey
})
.signers([adminKeypair])
.rpc();

console.log('All SOL withdrawn from treasury');
```

### Withdraw Unsold Tokens

After the presale ends, the admin can withdraw unsold tokens:

```javascript
// Create admin token account if not already created
const adminTokenAccount = await createAccount2022(
  connection,
  adminKeypair,
  tokenMint,
  adminKeypair.publicKey,
  undefined,
  undefined,
  TOKEN_2022_PROGRAM_ID
);

// Withdraw unsold tokens (can only be called after presale end)
await program.methods.withdrawUnsoldTokens()
.accounts({
  presale: presalePDA,
  admin: adminKeypair.publicKey,
  presaleTokenAccount: presaleTokenAccount,
  adminTokenAccount: adminTokenAccount,
  tokenMint: tokenMint,
  tokenProgram: TOKEN_2022_PROGRAM_ID
})
.signers([adminKeypair])
.rpc();

console.log('Unsold tokens withdrawn');
```

## Error Handling

The program includes various error checks to ensure proper operation:

- `PresaleNotActive`: Presale is not currently active
- `PresaleNotStarted`: Attempted to buy tokens before the start time
- `PresaleEnded`: Attempted to buy tokens after the end time
- `PresaleNotEnded`: Attempted to withdraw unsold tokens before the end time
- `BelowMinimumPurchase`: Purchase amount is below the minimum threshold
- `AboveMaximumPurchase`: Purchase amount exceeds the maximum threshold
- `CalculationError`: An arithmetic overflow/underflow occurred
- `Unauthorized`: The signer is not the admin
- `InsufficientBalance`: Attempted to withdraw more SOL than available

## Security Considerations

- The program uses PDAs to securely manage presale funds and tokens
- All critical operations require admin authorization
- Time-based constraints prevent unauthorized actions
- Safe arithmetic operations prevent integer overflows/underflows
- Account constraints ensure operations are performed on the correct accounts

## License

[MIT](LICENSE)
