#![no_std]

mod admin;
mod events;
mod types;

#[cfg(test)]
mod test;

use soroban_sdk::{contract, contractimpl, token, Address, Env, String, Vec};

use types::DataKey;
pub use types::{SafeTradeError, Trade, TradeStatus};

const DAY_IN_LEDGERS: u32 = 17280;
const INSTANCE_LIFETIME_THRESHOLD: u32 = DAY_IN_LEDGERS * 30;
const INSTANCE_BUMP_AMOUNT: u32 = DAY_IN_LEDGERS * 60;
const TRADE_LIFETIME_THRESHOLD: u32 = DAY_IN_LEDGERS * 30;
const TRADE_BUMP_AMOUNT: u32 = DAY_IN_LEDGERS * 60;

pub(crate) fn bump_instance(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
}

pub(crate) fn bump_trade(env: &Env, trade_id: u64) {
    env.storage().persistent().extend_ttl(
        &DataKey::Trade(trade_id),
        TRADE_LIFETIME_THRESHOLD,
        TRADE_BUMP_AMOUNT,
    );
}

#[contract]
pub struct SafeTradeContract;

#[contractimpl]
impl SafeTradeContract {
    pub fn create_trade(
        env: Env,
        buyer: Address,
        seller: Address,
        token: Address,
        amount: i128,
        deadline: u64,
        item: String,
    ) -> Result<u64, SafeTradeError> {
        buyer.require_auth();

        if amount <= 0 {
            return Err(SafeTradeError::InvalidAmount);
        }
        if deadline <= env.ledger().timestamp() {
            return Err(SafeTradeError::DeadlineInPast);
        }
        if buyer == seller {
            return Err(SafeTradeError::SameAddress);
        }

        token::Client::new(&env, &token).transfer(&buyer, &env.current_contract_address(), &amount);

        let trade_id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::NextId)
            .unwrap_or(0u64);
        env.storage()
            .instance()
            .set(&DataKey::NextId, &(trade_id + 1));
        bump_instance(&env);

        let trade = Trade {
            buyer: buyer.clone(),
            seller: seller.clone(),
            token,
            amount,
            deadline,
            status: TradeStatus::Funded,
            item: item.clone(),
        };
        env.storage()
            .persistent()
            .set(&DataKey::Trade(trade_id), &trade);
        bump_trade(&env, trade_id);

        events::trade_created(&env, trade_id, &buyer, &seller, amount, &item);
        Ok(trade_id)
    }

    pub fn confirm_receipt(env: Env, trade_id: u64, buyer: Address) -> Result<(), SafeTradeError> {
        buyer.require_auth();

        let mut trade: Trade = env
            .storage()
            .persistent()
            .get(&DataKey::Trade(trade_id))
            .ok_or(SafeTradeError::TradeNotFound)?;

        if trade.buyer != buyer {
            return Err(SafeTradeError::NotAuthorized);
        }
        if trade.status != TradeStatus::Funded {
            return Err(SafeTradeError::InvalidStatus);
        }

        token::Client::new(&env, &trade.token).transfer(
            &env.current_contract_address(),
            &trade.seller,
            &trade.amount,
        );
        trade.status = TradeStatus::Completed;
        env.storage()
            .persistent()
            .set(&DataKey::Trade(trade_id), &trade);
        bump_trade(&env, trade_id);

        events::trade_completed(&env, trade_id, &trade.seller, trade.amount);
        Ok(())
    }

    pub fn cancel_trade(env: Env, trade_id: u64, caller: Address) -> Result<(), SafeTradeError> {
        caller.require_auth();

        let mut trade: Trade = env
            .storage()
            .persistent()
            .get(&DataKey::Trade(trade_id))
            .ok_or(SafeTradeError::TradeNotFound)?;

        if trade.status != TradeStatus::Funded {
            return Err(SafeTradeError::InvalidStatus);
        }

        let is_buyer = trade.buyer == caller;
        let is_expired = env.ledger().timestamp() > trade.deadline;
        if !is_buyer && !is_expired {
            return Err(SafeTradeError::NotAuthorized);
        }

        token::Client::new(&env, &trade.token).transfer(
            &env.current_contract_address(),
            &trade.buyer,
            &trade.amount,
        );
        trade.status = TradeStatus::Cancelled;
        env.storage()
            .persistent()
            .set(&DataKey::Trade(trade_id), &trade);
        bump_trade(&env, trade_id);

        events::trade_cancelled(&env, trade_id, &trade.buyer, trade.amount);
        Ok(())
    }

    pub fn open_dispute(env: Env, trade_id: u64, buyer: Address) -> Result<(), SafeTradeError> {
        buyer.require_auth();

        let mut trade: Trade = env
            .storage()
            .persistent()
            .get(&DataKey::Trade(trade_id))
            .ok_or(SafeTradeError::TradeNotFound)?;

        if trade.buyer != buyer {
            return Err(SafeTradeError::NotAuthorized);
        }
        if trade.status != TradeStatus::Funded {
            return Err(SafeTradeError::InvalidStatus);
        }

        trade.status = TradeStatus::Disputed;
        env.storage()
            .persistent()
            .set(&DataKey::Trade(trade_id), &trade);
        bump_trade(&env, trade_id);

        events::trade_disputed(&env, trade_id, &buyer);
        Ok(())
    }

    pub fn initialize(env: Env, admin: Address) -> Result<(), SafeTradeError> {
        admin::initialize(env, admin)
    }

    pub fn resolve_dispute(
        env: Env,
        admin: Address,
        trade_id: u64,
        winner: Address,
    ) -> Result<(), SafeTradeError> {
        admin::resolve_dispute(env, admin, trade_id, winner)
    }

    pub fn transfer_admin(
        env: Env,
        admin: Address,
        new_admin: Address,
    ) -> Result<(), SafeTradeError> {
        admin::transfer_admin(env, admin, new_admin)
    }

    pub fn get_admin(env: Env) -> Result<Address, SafeTradeError> {
        admin::get_admin(env)
    }

    pub fn get_trade(env: Env, trade_id: u64) -> Result<Trade, SafeTradeError> {
        env.storage()
            .persistent()
            .get(&DataKey::Trade(trade_id))
            .ok_or(SafeTradeError::TradeNotFound)
    }

    pub fn trade_count(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::NextId)
            .unwrap_or(0u64)
    }

    pub fn get_trades_by_buyer(env: Env, buyer: Address) -> Vec<Trade> {
        let count = Self::trade_count(env.clone());
        let mut result = Vec::new(&env);
        for trade_id in 0..count {
            let stored: Option<Trade> = env.storage().persistent().get(&DataKey::Trade(trade_id));
            if let Some(trade) = stored {
                if trade.buyer == buyer {
                    result.push_back(trade);
                }
            }
        }
        result
    }

    pub fn get_trades_by_seller(env: Env, seller: Address) -> Vec<Trade> {
        let count = Self::trade_count(env.clone());
        let mut result = Vec::new(&env);
        for trade_id in 0..count {
            let stored: Option<Trade> = env.storage().persistent().get(&DataKey::Trade(trade_id));
            if let Some(trade) = stored {
                if trade.seller == seller {
                    result.push_back(trade);
                }
            }
        }
        result
    }
}
