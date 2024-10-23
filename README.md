# options_vault

**Options Vault** is a decentralized options trading vault built on the Solana blockchain. It allows users to deposit assets (e.g., SOL, USDC) into the vault, which then automatically executes predefined options trading strategies to generate yield. The vault also supports leveraged positions, time-locked staking rewards, and fee-based withdrawals.

The smart contract is written using the Anchor framework, and the core functionality includes depositing, withdrawing, staking, borrowing, and strategy execution with automated options trading strategies like selling covered calls or cash-secured puts.

## Features

- **Automated Options Strategies**: Predefined strategies such as covered calls and cash-secured puts are executed based on market conditions.
- **Leverage**: Users can borrow against their collateral to take leveraged positions within the vault.
- **Staking & Rewards**: Users earn boosted rewards based on the duration of their staked assets.
- **Withdrawals**: Users can withdraw assets with a fee or perform emergency withdrawals without a fee.
- **Pause/Unpause Vault**: The vault can be paused by the admin, preventing deposits and withdrawals until it is unpaused.
- **Liquidation**: If a user's collateral falls below the liquidation threshold, their position can be automatically liquidated.
- **Strategy Execution Frequency Control**: Options strategies can only be executed at a certain interval to prevent abuse or frequent execution.
