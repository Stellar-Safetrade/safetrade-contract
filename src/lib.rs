#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, token, Address, Env, Symbol};

const TRADE: Symbol = symbol_short!("TRADE");

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum TradeStatus { Funded, Completed, Disputed, Cancelled }

#[contracttype]
#[derive(Clone, Debug)]
pub struct Trade {
    pub buyer: Address,
    pub seller: Address,
    pub token: Address,
    pub amount: i128,
    pub deadline: u64,
    pub status: TradeStatus,
    pub item: soroban_sdk::String,
}

#[contracttype]
pub enum DataKey { Trade(u64), NextId }

#[contract]
pub struct SafeTradeContract;

#[contractimpl]
impl SafeTradeContract {
    pub fn create_trade(env: Env, buyer: Address, seller: Address, token: Address, amount: i128, deadline: u64, item: soroban_sdk::String) -> u64 {
        buyer.require_auth();
        if amount <= 0 { panic!("amount must be greater than zero"); }
        if deadline <= env.ledger().timestamp() { panic!("deadline must be in the future"); }
        if buyer == seller { panic!("buyer and seller cannot be the same"); }
        token::Client::new(&env, &token).transfer(&buyer, &env.current_contract_address(), &amount);
        let trade_id: u64 = env.storage().instance().get(&DataKey::NextId).unwrap_or(0u64);
        env.storage().instance().set(&DataKey::NextId, &(trade_id + 1));
        env.storage().persistent().set(&DataKey::Trade(trade_id), &Trade { buyer, seller, token, amount, deadline, status: TradeStatus::Funded, item });
        env.events().publish((TRADE, symbol_short!("created")), trade_id);
        trade_id
    }

    pub fn confirm_receipt(env: Env, trade_id: u64, buyer: Address) {
        buyer.require_auth();
        let mut trade: Trade = env.storage().persistent().get(&DataKey::Trade(trade_id)).expect("trade not found");
        if trade.buyer != buyer { panic!("only buyer can confirm"); }
        if trade.status != TradeStatus::Funded { panic!("trade not funded"); }
        token::Client::new(&env, &trade.token).transfer(&env.current_contract_address(), &trade.seller, &trade.amount);
        trade.status = TradeStatus::Completed;
        env.storage().persistent().set(&DataKey::Trade(trade_id), &trade);
        env.events().publish((TRADE, symbol_short!("complete")), trade_id);
    }

    pub fn cancel_trade(env: Env, trade_id: u64, caller: Address) {
        caller.require_auth();
        let mut trade: Trade = env.storage().persistent().get(&DataKey::Trade(trade_id)).expect("trade not found");
        if trade.status != TradeStatus::Funded { panic!("trade cannot be cancelled"); }
        let is_buyer = trade.buyer == caller;
        let is_expired = env.ledger().timestamp() > trade.deadline;
        if !is_buyer && !is_expired { panic!("only buyer can cancel before deadline"); }
        token::Client::new(&env, &trade.token).transfer(&env.current_contract_address(), &trade.buyer, &trade.amount);
        trade.status = TradeStatus::Cancelled;
        env.storage().persistent().set(&DataKey::Trade(trade_id), &trade);
        env.events().publish((TRADE, symbol_short!("cancel")), trade_id);
    }

    pub fn open_dispute(env: Env, trade_id: u64, buyer: Address) {
        buyer.require_auth();
        let mut trade: Trade = env.storage().persistent().get(&DataKey::Trade(trade_id)).expect("trade not found");
        if trade.buyer != buyer { panic!("only buyer can dispute"); }
        if trade.status != TradeStatus::Funded { panic!("trade not funded"); }
        trade.status = TradeStatus::Disputed;
        env.storage().persistent().set(&DataKey::Trade(trade_id), &trade);
        env.events().publish((TRADE, symbol_short!("dispute")), trade_id);
    }

    pub fn get_trade(env: Env, trade_id: u64) -> Trade {
        env.storage().persistent().get(&DataKey::Trade(trade_id)).expect("trade not found")
    }

    pub fn trade_count(env: Env) -> u64 {
        env.storage().instance().get(&DataKey::NextId).unwrap_or(0u64)
    }
}
