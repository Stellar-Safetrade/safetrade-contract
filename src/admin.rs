use soroban_sdk::{token, Address, Env};

use crate::events;
use crate::types::{DataKey, SafeTradeError, Trade, TradeStatus};
use crate::{bump_instance, bump_trade};

fn require_admin(env: &Env, caller: &Address) -> Result<(), SafeTradeError> {
    let stored: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(SafeTradeError::NotAuthorized)?;
    if &stored != caller {
        return Err(SafeTradeError::NotAuthorized);
    }
    Ok(())
}

pub fn initialize(env: Env, admin: Address) -> Result<(), SafeTradeError> {
    if env.storage().instance().has(&DataKey::Admin) {
        return Err(SafeTradeError::AlreadyInitialized);
    }
    admin.require_auth();

    env.storage().instance().set(&DataKey::Admin, &admin);
    bump_instance(&env);
    Ok(())
}

pub fn resolve_dispute(
    env: Env,
    admin: Address,
    trade_id: u64,
    winner: Address,
) -> Result<(), SafeTradeError> {
    require_admin(&env, &admin)?;
    admin.require_auth();

    let mut trade: Trade = env
        .storage()
        .persistent()
        .get(&DataKey::Trade(trade_id))
        .ok_or(SafeTradeError::TradeNotFound)?;

    if trade.status != TradeStatus::Disputed {
        return Err(SafeTradeError::InvalidStatus);
    }
    if winner != trade.buyer && winner != trade.seller {
        return Err(SafeTradeError::InvalidWinner);
    }

    token::Client::new(&env, &trade.token).transfer(
        &env.current_contract_address(),
        &winner,
        &trade.amount,
    );
    trade.status = TradeStatus::Completed;
    env.storage()
        .persistent()
        .set(&DataKey::Trade(trade_id), &trade);
    bump_trade(&env, trade_id);

    events::dispute_resolved(&env, trade_id, &winner, trade.amount);
    Ok(())
}

pub fn transfer_admin(env: Env, admin: Address, new_admin: Address) -> Result<(), SafeTradeError> {
    require_admin(&env, &admin)?;
    admin.require_auth();

    env.storage().instance().set(&DataKey::Admin, &new_admin);
    bump_instance(&env);
    Ok(())
}

pub fn get_admin(env: Env) -> Result<Address, SafeTradeError> {
    env.storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(SafeTradeError::NotAuthorized)
}
