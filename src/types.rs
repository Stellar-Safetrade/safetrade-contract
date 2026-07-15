use soroban_sdk::{contracterror, contracttype, Address, String};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TradeStatus {
    Funded,
    Completed,
    Disputed,
    Cancelled,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct Trade {
    pub buyer: Address,
    pub seller: Address,
    pub token: Address,
    pub amount: i128,
    pub deadline: u64,
    pub status: TradeStatus,
    pub item: String,
}

#[contracttype]
pub enum DataKey {
    Admin,
    Trade(u64),
    NextId,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum SafeTradeError {
    TradeNotFound = 1,
    NotAuthorized = 2,
    InvalidAmount = 3,
    DeadlineInPast = 4,
    SameAddress = 5,
    InvalidStatus = 6,
    AlreadyInitialized = 7,
    InvalidWinner = 8,
}
