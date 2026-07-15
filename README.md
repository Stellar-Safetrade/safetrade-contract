# SafeTrade Escrow Contract

A simple peer-to-peer escrow smart contract for the Stellar network built with Soroban. It enables secure on-chain escrow flows for buyers and sellers by holding funds in escrow until the buyer confirms receipt or the trade is cancelled or disputed.

## Overview

SafeTrade is designed for trust-minimized trade settlement between two parties. The contract manages:

- trade creation with escrowed funds
- buyer confirmation of delivery
- seller or buyer cancellation under defined conditions
- dispute initiation for unresolved trades
- trade state tracking and retrieval

## Contract Features

### Core capabilities

- Create a trade with buyer, seller, token, amount, deadline, and item description
- Lock funds in the contract until the trade reaches a final state
- Release funds to the seller on successful confirmation
- Refund funds to the buyer on cancellation
- Mark a trade as disputed when the buyer raises an issue

### Trade lifecycle

A trade progresses through the following states:

- Funded: initial state after escrow funds are locked
- Completed: funds were released to the seller after confirmation
- Disputed: the buyer opened a dispute
- Cancelled: the trade was cancelled and funds were refunded

## Contract Interface

The contract exposes the following entry points:

- create_trade(...) creates a new escrow trade
- confirm_receipt(...) allows the buyer to confirm receipt and release funds
- cancel_trade(...) allows the buyer to cancel before the deadline or lets the deadline expiry trigger cancellation
- open_dispute(...) lets the buyer mark the trade as disputed
- get_trade(...) returns the trade details for a given ID
- trade_count(...) returns the total number of trades created
- get_trades_by_buyer(buyer) returns all trades for a given buyer
- get_trades_by_seller(seller) returns all trades for a given seller

Admin / dispute resolution:

- initialize(admin) sets the contract's admin (one-time only)
- resolve_dispute(admin, trade_id, winner) lets the admin settle a disputed
  trade by releasing the escrowed funds to either the buyer or the seller
- transfer_admin(admin, new_admin) hands off the admin role
- get_admin() returns the current admin address

Every state-changing entry point returns `Result<_, SafeTradeError>` instead
of panicking on expected error conditions:

| Code | Error                | Meaning                                     |
|------|-----------------------|----------------------------------------------|
| 1    | `TradeNotFound`       | no trade with that id                        |
| 2    | `NotAuthorized`       | caller isn't allowed to perform this action  |
| 3    | `InvalidAmount`       | amount must be greater than zero             |
| 4    | `DeadlineInPast`      | deadline must be in the future               |
| 5    | `SameAddress`         | buyer and seller cannot be the same          |
| 6    | `InvalidStatus`       | trade isn't in the required status           |
| 7    | `AlreadyInitialized`  | admin already set                            |
| 8    | `InvalidWinner`       | dispute winner must be the buyer or seller   |

Every state change also emits an event (`TradeCreated`, `TradeCompleted`,
`TradeCancelled`, `TradeDisputed`, `DisputeResolved`) under the `TRADE` topic,
defined in `src/events.rs`.

## Data Model

The contract stores:

- Trade: contains buyer, seller, token, amount, deadline, status, and item details
- DataKey::Trade(u64): persistent storage for each trade by ID
- DataKey::NextId: tracks the next trade ID to assign
- DataKey::Admin: instance storage holding the current admin address

Persistent trade records and instance storage have their TTL extended
(`extend_ttl`) on every write, so contract state doesn't archive out from
under active trades.

## Project Layout

```
src/
├── lib.rs      contract entrypoints (trade lifecycle + admin delegation)
├── admin.rs    admin initialization, dispute resolution, admin transfer
├── events.rs   contract event structs + topic constants
├── types.rs    Trade, TradeStatus, DataKey, SafeTradeError
└── test.rs     unit tests
```

## Usage Flow

1. The buyer creates a trade and funds are transferred into the contract escrow.
2. The seller fulfills the agreement off-chain.
3. The buyer confirms receipt, triggering release of escrowed funds to the seller.
4. If the trade is not resolved, the buyer may open a dispute or the trade may be cancelled after the deadline.

## Build & Test

A `Makefile` wraps the common commands:

```bash
make build           # cargo build --target wasm32-unknown-unknown --release
make test            # cargo test
make fmt             # cargo fmt
make lint            # cargo clippy
make deploy-testnet  # deploy the built wasm to Stellar testnet
```

`deploy-testnet` expects a funded testnet account; edit `SOURCE_ACCOUNT` in
the `Makefile` (or override on the command line, e.g.
`make deploy-testnet SOURCE_ACCOUNT=alice`) before deploying.

CI (`.github/workflows/test.yml`) runs `cargo fmt --check`, `cargo clippy`,
the wasm build, and `cargo test` on every push and pull request.

## Development Notes

This contract uses Soroban SDK version 21.0.0 and is compiled as a Soroban CDylib.

### Prerequisites

- Rust
- Cargo
- Soroban toolchain support for Stellar smart contracts

## Security Considerations

- Buyer authentication is required for trade creation, receipt confirmation, and dispute creation.
- Funds are transferred only under explicitly validated conditions.
- The contract checks for invalid amounts, invalid deadlines, and self-trades.
- Cancellation is restricted to the buyer before the deadline and is allowed after the deadline expiration.

## Notes

This README provides a high-level overview of the contract. For production usage, review the contract logic carefully and consider adding additional governance, dispute resolution, or administrative safeguards.
