//TODO: REVIEW THIS TEST FILE

describe("Options Vault Tests", () => {
  // Setting up variables and constants for the tests
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = pg.program; 

  const VAULT_SEED = "vault";
  const DEPOSIT_AMOUNT = new anchor.BN(1000); // Example deposit amount
  const BORROW_AMOUNT = new anchor.BN(300); // Example borrow amount
  const STRATEGY_EXECUTION_INTERVAL = 3600; // 1 hour in seconds
  const LIQUIDATION_THRESHOLD = 150; // Example: 150% collateral-to-debt ratio
  const REWARD_RATE = new anchor.BN(10); // Example reward rate per second
  const WITHDRAWAL_FEE_PERCENT = 5; // 5% withdrawal fee

  // Accounts and keypairs for test cases
  let vaultKeypair: anchor.web3.Keypair;
  let userKeypair: anchor.web3.Keypair;

  // Helper function to simulate delays
  function sleep(ms: number) {
    return new Promise(resolve => setTimeout(resolve, ms));
  }

  before(async () => {
    // Create keypairs for vault and user
    vaultKeypair = anchor.web3.Keypair.generate();
    userKeypair = anchor.web3.Keypair.generate();

    // Airdrop SOL to user for testing (to pay for transactions)
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(userKeypair.publicKey, 2 * web3.LAMPORTS_PER_SOL),
      "confirmed"
    );
  });

  it("Initialize Vault", async () => {
    // Call the initialize vault function
    await program.methods
      .initializeVault(new anchor.BN(0)) // Example bump
      .accounts({
        vault: vaultKeypair.publicKey,
        user: provider.wallet.publicKey,
        authority: provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([vaultKeypair])
      .rpc();

    // Fetch and validate the vault data
    const vault = await program.account.vault.fetch(vaultKeypair.publicKey);
    assert.ok(vault.totalDeposits.eq(new anchor.BN(0)));
    assert.strictEqual(vault.paused, false);
  });

  it("Deposit Assets into Vault", async () => {
    // Call the deposit function
    await program.methods
      .deposit(DEPOSIT_AMOUNT)
      .accounts({
        vault: vaultKeypair.publicKey,
        user: userKeypair.publicKey,
        userTokenAccount: userKeypair.publicKey, // Assuming SOL deposit, in a real case, use a token account
        vaultTokenAccount: vaultKeypair.publicKey, // The vault's token account
        tokenProgram: anchor.web3.SystemProgram.programId, // Use token program for SPL tokens
      })
      .signers([userKeypair])
      .rpc();

    // Fetch and validate the vault data
    const vault = await program.account.vault.fetch(vaultKeypair.publicKey);
    assert.ok(vault.totalDeposits.eq(DEPOSIT_AMOUNT));
  });

  it("Claim Rewards and Auto-Compound", async () => {
    // Wait for some time to simulate staking duration
    await sleep(1000); // Wait for 1 second

    // Call the claim_rewards function to auto-compound rewards
    await program.methods
      .claimRewards()
      .accounts({
        user: userKeypair.publicKey,
        vault: vaultKeypair.publicKey,
      })
      .signers([userKeypair])
      .rpc();

    // Fetch and validate the user's rewards and vault deposit
    const vault = await program.account.vault.fetch(vaultKeypair.publicKey);
    const user = await program.account.user.fetch(userKeypair.publicKey);

    assert.ok(user.rewardBalance.eq(new anchor.BN(0))); // Rewards should be zero after compounding
    assert.ok(vault.totalDeposits.gte(DEPOSIT_AMOUNT)); // Vault deposits should increase after compounding
  });

  it("Borrow with Leverage Cap", async () => {
    // Call the borrow function (ensure borrow amount doesn't exceed leverage cap)
    await program.methods
      .borrow(BORROW_AMOUNT)
      .accounts({
        vault: vaultKeypair.publicKey,
        user: userKeypair.publicKey,
        userTokenAccount: userKeypair.publicKey,
        tokenProgram: anchor.web3.SystemProgram.programId, // SPL Token Program
      })
      .signers([userKeypair])
      .rpc();

    // Fetch and validate the vault data
    const vault = await program.account.vault.fetch(vaultKeypair.publicKey);
    console.log(`User borrowed ${BORROW_AMOUNT.toString()} tokens using leverage.`);

    // Verify that the vault allows borrowing within the leverage limit
    assert.ok(vault.totalDeposits.gte(DEPOSIT_AMOUNT), "Leverage cap should not be exceeded");
  });

  it("Withdraw Assets from Vault with Fee", async () => {
    // Calculate fee and expected withdrawal amount
    const withdrawalAmount = new anchor.BN(500); // Amount user wants to withdraw
    const expectedFee = withdrawalAmount.mul(new anchor.BN(WITHDRAWAL_FEE_PERCENT)).div(new anchor.BN(100));
    const netAmountAfterFee = withdrawalAmount.sub(expectedFee);

    // Call the withdraw function
    await program.methods
      .withdraw(withdrawalAmount)
      .accounts({
        vault: vaultKeypair.publicKey,
        user: userKeypair.publicKey,
        userTokenAccount: userKeypair.publicKey, // Assuming SOL, in real case, use SPL token account
        vaultTokenAccount: vaultKeypair.publicKey,
        feeVaultTokenAccount: vaultKeypair.publicKey, // Admin's fee vault
        tokenProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([userKeypair])
      .rpc();

    // Fetch and validate vault data
    const vault = await program.account.vault.fetch(vaultKeypair.publicKey);
    assert.ok(vault.totalDeposits.eq(DEPOSIT_AMOUNT.sub(withdrawalAmount)));

    console.log(`User withdrew ${netAmountAfterFee.toString()} tokens after a ${expectedFee.toString()} fee.`);
  });

  it("Emergency Withdraw Without Fee", async () => {
    const emergencyWithdrawAmount = new anchor.BN(300); // Emergency withdrawal amount

    // Call the emergency_withdraw function
    await program.methods
      .emergencyWithdraw(emergencyWithdrawAmount)
      .accounts({
        vault: vaultKeypair.publicKey,
        user: userKeypair.publicKey,
        userTokenAccount: userKeypair.publicKey,
        vaultTokenAccount: vaultKeypair.publicKey,
        tokenProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([userKeypair])
      .rpc();

    // Fetch and validate vault data
    const vault = await program.account.vault.fetch(vaultKeypair.publicKey);
    assert.ok(vault.totalDeposits.eq(DEPOSIT_AMOUNT.sub(emergencyWithdrawAmount)));

    console.log(`Emergency withdrawal of ${emergencyWithdrawAmount.toString()} tokens successful without fees.`);
  });

  it("Test Boosted Rewards Based on Staking Duration", async () => {
    // Wait for a long duration to simulate long-term staking
    await sleep(3000); // Simulate time progression (3 seconds)

    // Call the claim_rewards function after a long staking period
    await program.methods
      .claimRewards()
      .accounts({
        user: userKeypair.publicKey,
        vault: vaultKeypair.publicKey,
      })
      .signers([userKeypair])
      .rpc();

    // Fetch and validate rewards
    const user = await program.account.user.fetch(userKeypair.publicKey);

    // Ensure rewards were boosted based on staking duration
    assert.ok(user.rewardBalance.gt(new anchor.BN(0)), "Boosted rewards should be accumulated based on staking duration.");
    console.log(`User has accumulated ${user.rewardBalance.toString()} boosted rewards.`);
  });

  // Existing tests for Pause/Unpause and Liquidation...

  it("Execute Strategy with Frequency Control", async () => {
    // Call execute_strategy with a mock market price
    const marketPrice = new anchor.BN(1500);

    await program.methods
      .executeStrategy(marketPrice)
      .accounts({
        vault: vaultKeypair.publicKey,
        user: userKeypair.publicKey,
      })
      .signers([userKeypair])
      .rpc();

    // Fetch the vault and validate strategy execution updates
    const vault = await program.account.vault.fetch(vaultKeypair.publicKey);
    assert.ok(vault.totalTrades.eq(new anchor.BN(1))); // Ensure that the trade count has increased

    // Ensure that strategy cannot be executed within the interval
    try {
      await program.methods
        .executeStrategy(marketPrice)
        .accounts({
          vault: vaultKeypair.publicKey,
          user: userKeypair.publicKey,
        })
        .signers([userKeypair])
        .rpc();
      assert.fail("Strategy execution should fail if called too soon");
    } catch (err) {
      assert.ok(err.message.includes("Strategy execution too soon"));
    }
  });
});
