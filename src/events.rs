use soroban_sdk::{contracttype, symbol_short, Address, Env, String, Symbol};

const TRADE: Symbol = symbol_short!("TRADE");

const CREATED: Symbol = symbol_short!("created");
const COMPLETED: Symbol = symbol_short!("complete");
const CANCELLED: Symbol = symbol_short!("cancel");
const DISPUTED: Symbol = symbol_short!("dispute");
const RESOLVED: Symbol = symbol_short!("resolved");

#[contracttype]
#[derive(Clone, Debug)]
pub struct TradeCreated {
    pub trade_id: u64,
    pub buyer: Address,
    pub seller: Address,
    pub amount: i128,
    pub item: String,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct TradeCompleted {
    pub trade_id: u64,
    pub seller: Address,
    pub amount: i128,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct TradeCancelled {
    pub trade_id: u64,
    pub buyer: Address,
    pub amount: i128,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct TradeDisputed {
    pub trade_id: u64,
    pub buyer: Address,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct DisputeResolved {
    pub trade_id: u64,
    pub winner: Address,
    pub amount: i128,
}

pub fn trade_created(
    env: &Env,
    trade_id: u64,
    buyer: &Address,
    seller: &Address,
    amount: i128,
    item: &String,
) {
    env.events().publish(
        (TRADE, CREATED),
        TradeCreated {
            trade_id,
            buyer: buyer.clone(),
            seller: seller.clone(),
            amount,
            item: item.clone(),
        },
    );
}

pub fn trade_completed(env: &Env, trade_id: u64, seller: &Address, amount: i128) {
    env.events().publish(
        (TRADE, COMPLETED),
        TradeCompleted {
            trade_id,
            seller: seller.clone(),
            amount,
        },
    );
}

pub fn trade_cancelled(env: &Env, trade_id: u64, buyer: &Address, amount: i128) {
    env.events().publish(
        (TRADE, CANCELLED),
        TradeCancelled {
            trade_id,
            buyer: buyer.clone(),
            amount,
        },
    );
}

pub fn trade_disputed(env: &Env, trade_id: u64, buyer: &Address) {
    env.events().publish(
        (TRADE, DISPUTED),
        TradeDisputed {
            trade_id,
            buyer: buyer.clone(),
        },
    );
}

pub fn dispute_resolved(env: &Env, trade_id: u64, winner: &Address, amount: i128) {
    env.events().publish(
        (TRADE, RESOLVED),
        DisputeResolved {
            trade_id,
            winner: winner.clone(),
            amount,
        },
    );
}
