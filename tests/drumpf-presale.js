const anchor = require('@project-serum/anchor');
const { BN, web3, Program } = anchor;
const { SystemProgram } = web3;
const { TOKEN_PROGRAM_ID } = require('@solana/spl-token');
const { TOKEN_2022_PROGRAM_ID, createMint: createMint2022, createAccount: createAccount2022, mintTo: mintTo2022 } = require('@solana/spl-token-2022');
const assert = require('assert');

describe('solana-presale-token2022', () => {
  // Configure the client to use the local cluster
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.SolanaPresale;
  
  // Generate necessary keypairs
  const adminWallet = anchor.web3.Keypair.generate();
  const buyerWallet = anchor.web3.Keypair.generate();
  const treasuryWallet = anchor.web3.Keypair.generate();
  
  // Program data
  let tokenMint;
  let presaleTokenAccount;
  let buyerTokenAccount;
  let adminTokenAccount;
  let presalePDA;
  let presaleBump;
  
  // Presale parameters
  const rate = new BN(1000); // 1000 tokens per SOL
  const presaleStart = new BN(Math.floor(Date.now() / 1000)); // now
  const presaleEnd = new BN(Math.floor(Date.now() / 1000) + 86400); // 1 day from now
  const minPurchase = new BN(web3.LAMPORTS_PER_SOL / 10); // 0.1 SOL
  const maxPurchase = new BN(web3.LAMPORTS_PER_SOL * 10); // 10 SOL
  
  before(async () => {
    // Fund admin and buyer wallets
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(adminWallet.publicKey, web3.LAMPORTS_PER_SOL * 100)
    );
    
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(buyerWallet.publicKey, web3.LAMPORTS_PER_SOL * 100)
    );
    
    // Create token mint using Token-2022 program
    tokenMint = await createMint2022(
      provider.connection,
      adminWallet,
      adminWallet.publicKey,
      null,
      9, // 9 decimals
      undefined,
      undefined,
      TOKEN_2022_PROGRAM_ID
    );
    
    // Find presale PDA
    [presalePDA, presaleBump] = await web3.PublicKey.findProgramAddress(
      [Buffer.from('presale')],
      program.programId
    );
    
    // Create presale token account (owned by the PDA) using Token-2022
    presaleTokenAccount = await createAccount2022(
      provider.connection,
      adminWallet,
      tokenMint,
      presalePDA,
      undefined,
      undefined,
      TOKEN_2022_PROGRAM_ID
    );
    
    // Create buyer's token account using Token-2022
    buyerTokenAccount = await createAccount2022(
      provider.connection,
      adminWallet, // payer
      tokenMint,
      buyerWallet.publicKey,
      undefined,
      undefined,
      TOKEN_2022_PROGRAM_ID
    );
    
    // Create admin's token account using Token-2022
    adminTokenAccount = await createAccount2022(
      provider.connection,
      adminWallet,
      tokenMint,
      adminWallet.publicKey,
      undefined,
      undefined,
      TOKEN_2022_PROGRAM_ID
    );
    
    // Mint tokens to the presale account using Token-2022
    await mintTo2022(
      provider.connection,
      adminWallet,
      tokenMint,
      presaleTokenAccount,
      adminWallet.publicKey,
      1_000_000 * 10**9, // 1 million tokens
      [],
      undefined,
      TOKEN_2022_PROGRAM_ID
    );
  });

  it('Initializes the presale', async () => {
    await program.methods.initialize(
      rate,
      presaleStart,
      presaleEnd,
      minPurchase,
      maxPurchase
    )
    .accounts({
      presale: presalePDA,
      admin: adminWallet.publicKey,
      tokenMint: tokenMint,
      treasury: treasuryWallet.publicKey,
      presaleTokenAccount: presaleTokenAccount,
      systemProgram: SystemProgram.programId,
      tokenProgram: TOKEN_2022_PROGRAM_ID, // Use Token-2022 program ID
      rent: web3.SYSVAR_RENT_PUBKEY
    })
    .signers([adminWallet])
    .rpc();
    
    // Fetch the presale account and verify initialization
    const presaleAccount = await program.account.presale.fetch(presalePDA);
    assert.equal(presaleAccount.admin.toString(), adminWallet.publicKey.toString());
    assert.equal(presaleAccount.tokenMint.toString(), tokenMint.toString());
    assert.equal(presaleAccount.treasury.toString(), treasuryWallet.publicKey.toString());
    assert.equal(presaleAccount.presaleTokenAccount.toString(), presaleTokenAccount.toString());
    assert.equal(presaleAccount.rate.toString(), rate.toString());
    assert.equal(presaleAccount.presaleStart.toString(), presaleStart.toString());
    assert.equal(presaleAccount.presaleEnd.toString(), presaleEnd.toString());
    assert.equal(presaleAccount.minPurchase.toString(), minPurchase.toString());
    assert.equal(presaleAccount.maxPurchase.toString(), maxPurchase.toString());
    assert.equal(presaleAccount.totalSold.toString(), '0');
    assert.equal(presaleAccount.isActive, true);
  });

  it('Buys tokens', async () => {
    const purchaseAmount = web3.LAMPORTS_PER_SOL; // 1 SOL
    const expectedTokenAmount = purchaseAmount * rate.toNumber(); // 1000 tokens
    
    // Get initial balances
    const initialTreasuryBalance = await provider.connection.getBalance(treasuryWallet.publicKey);
    const initialBuyerTokenBalance = (await provider.connection.getTokenAccountBalance(buyerTokenAccount)).value.amount;
    
    await program.methods.buyTokens(new BN(purchaseAmount))
    .accounts({
      presale: presalePDA,
      buyer: buyerWallet.publicKey,
      treasury: treasuryWallet.publicKey,
      presaleTokenAccount: presaleTokenAccount,
      buyerTokenAccount: buyerTokenAccount,
      tokenMint: tokenMint,
      systemProgram: SystemProgram.programId,
      tokenProgram: TOKEN_2022_PROGRAM_ID // Use Token-2022 program ID
    })
    .signers([buyerWallet])
    .rpc();
    
    // Verify balances after purchase
    const finalTreasuryBalance = await provider.connection.getBalance(treasuryWallet.publicKey);
    const finalBuyerTokenBalance = (await provider.connection.getTokenAccountBalance(buyerTokenAccount)).value.amount;
    
    // Treasury received SOL
    assert.equal(finalTreasuryBalance - initialTreasuryBalance, purchaseAmount);
    
    // Buyer received tokens
    assert.equal(finalBuyerTokenBalance - initialBuyerTokenBalance, expectedTokenAmount.toString());
    
    // Total sold was updated
    const updatedPresale = await program.account.presale.fetch(presalePDA);
    assert.equal(updatedPresale.totalSold.toString(), expectedTokenAmount.toString());
  });

  it('Toggles presale status', async () => {
    // First deactivate
    await program.methods.togglePresale(false)
    .accounts({
      presale: presalePDA,
      admin: adminWallet.publicKey
    })
    .signers([adminWallet])
    .rpc();
    
    // Verify status
    let presaleAccount = await program.account.presale.fetch(presalePDA);
    assert.equal(presaleAccount.isActive, false);
    
    // Try to buy tokens when inactive (should fail)
    try {
      await program.methods.buyTokens(new BN(web3.LAMPORTS_PER_SOL))
      .accounts({
        presale: presalePDA,
        buyer: buyerWallet.publicKey,
        treasury: treasuryWallet.publicKey,
        presaleTokenAccount: presaleTokenAccount,
        buyerTokenAccount: buyerTokenAccount,
        tokenMint: tokenMint,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_2022_PROGRAM_ID // Use Token-2022 program ID
      })
      .signers([buyerWallet])
      .rpc();
      
      assert.fail("Expected transaction to fail when presale is inactive");
    } catch (error) {
      assert.ok(error.toString().includes("Presale is not active"));
    }
    
    // Activate again
    await program.methods.togglePresale(true)
    .accounts({
      presale: presalePDA,
      admin: adminWallet.publicKey
    })
    .signers([adminWallet])
    .rpc();
    
    // Verify status
    presaleAccount = await program.account.presale.fetch(presalePDA);
    assert.equal(presaleAccount.isActive, true);
  });

  it('Cannot withdraw tokens before presale ends', async () => {
    try {
      await program.methods.withdrawUnsoldTokens()
      .accounts({
        presale: presalePDA,
        admin: adminWallet.publicKey,
        presaleTokenAccount: presaleTokenAccount,
        adminTokenAccount: adminTokenAccount,
        tokenMint: tokenMint,
        tokenProgram: TOKEN_2022_PROGRAM_ID // Use Token-2022 program ID
      })
      .signers([adminWallet])
      .rpc();
      
      assert.fail("Expected transaction to fail when withdrawing tokens before presale end");
    } catch (error) {
      assert.ok(error.toString().includes("Presale has not ended yet"));
    }
  });

  it('Withdraws SOL from treasury', async () => {
    // Get initial balances
    const initialTreasuryBalance = await provider.connection.getBalance(treasuryWallet.publicKey);
    const initialAdminBalance = await provider.connection.getBalance(adminWallet.publicKey);
    
    // Withdraw half of the SOL
    const withdrawAmount = Math.floor(initialTreasuryBalance / 2);
    
    await program.methods.withdrawSol(new BN(withdrawAmount))
    .accounts({
      presale: presalePDA,
      admin: adminWallet.publicKey,
      treasury: treasuryWallet.publicKey
    })
    .signers([adminWallet])
    .rpc();
    
    // Verify balances
    const finalTreasuryBalance = await provider.connection.getBalance(treasuryWallet.publicKey);
    const finalAdminBalance = await provider.connection.getBalance(adminWallet.publicKey);
    
    // Account for transaction fees
    assert.ok(finalAdminBalance > initialAdminBalance);
    assert.equal(initialTreasuryBalance - finalTreasuryBalance, withdrawAmount);
    
    // Withdraw remaining SOL
    await program.methods.withdrawSol(null)
    .accounts({
      presale: presalePDA,
      admin: adminWallet.publicKey,
      treasury: treasuryWallet.publicKey
    })
    .signers([adminWallet])
    .rpc();
    
    // Verify treasury is empty
    const emptyTreasuryBalance = await provider.connection.getBalance(treasuryWallet.publicKey);
    assert.equal(emptyTreasuryBalance, 0);
  });

  // Note: To properly test withdraw_unsold_tokens, we would need to modify the program
  // or wait for the actual presale end time
});
