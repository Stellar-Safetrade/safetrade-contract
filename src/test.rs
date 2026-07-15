#![cfg(test)]

use super::*;
use soroban_sdk::testutils::{Address as _, Ledger as _};

fn create_funded_token(env: &Env, admin: &Address, to: &Address, amount: i128) -> Address {
    let sac = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    token::StellarAssetClient::new(env, &sac).mint(to, &amount);
    sac
}

struct Ctx {
    env: Env,
    admin: Address,
    buyer: Address,
    seller: Address,
    token: Address,
    contract_id: Address,
}

fn setup() -> Ctx {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let buyer = Address::generate(&env);
    let seller = Address::generate(&env);
    let token = create_funded_token(&env, &admin, &buyer, 1_000_000);

    let contract_id = env.register_contract(None, SafeTradeContract);
    let client = SafeTradeContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    Ctx {
        env,
        admin,
        buyer,
        seller,
        token,
        contract_id,
    }
}

fn item(env: &Env) -> String {
    String::from_str(env, "Vintage Camera")
}

#[test]
fn test_initialize_sets_admin() {
    let ctx = setup();
    let client = SafeTradeContractClient::new(&ctx.env, &ctx.contract_id);
    assert_eq!(client.get_admin(), ctx.admin);
}

#[test]
#[should_panic]
fn test_initialize_twice_fails() {
    let ctx = setup();
    let client = SafeTradeContractClient::new(&ctx.env, &ctx.contract_id);
    client.initialize(&ctx.admin);
}

#[test]
fn test_transfer_admin() {
    let ctx = setup();
    let client = SafeTradeContractClient::new(&ctx.env, &ctx.contract_id);
    let new_admin = Address::generate(&ctx.env);

    client.transfer_admin(&ctx.admin, &new_admin);
    assert_eq!(client.get_admin(), new_admin);
}

#[test]
#[should_panic]
fn test_transfer_admin_not_admin_fails() {
    let ctx = setup();
    let client = SafeTradeContractClient::new(&ctx.env, &ctx.contract_id);
    let impostor = Address::generate(&ctx.env);
    let new_admin = Address::generate(&ctx.env);

    client.transfer_admin(&impostor, &new_admin);
}

#[test]
fn test_create_trade_and_confirm_receipt() {
    let ctx = setup();
    let client = SafeTradeContractClient::new(&ctx.env, &ctx.contract_id);
    let token_client = token::Client::new(&ctx.env, &ctx.token);
    let deadline = ctx.env.ledger().timestamp() + 1_000;

    let trade_id = client.create_trade(
        &ctx.buyer,
        &ctx.seller,
        &ctx.token,
        &500,
        &deadline,
        &item(&ctx.env),
    );
    assert_eq!(trade_id, 0);
    assert_eq!(client.trade_count(), 1);
    assert_eq!(token_client.balance(&ctx.buyer), 1_000_000 - 500);
    assert_eq!(token_client.balance(&ctx.contract_id), 500);

    client.confirm_receipt(&trade_id, &ctx.buyer);

    let trade = client.get_trade(&trade_id);
    assert_eq!(trade.status, TradeStatus::Completed);
    assert_eq!(token_client.balance(&ctx.seller), 500);
    assert_eq!(token_client.balance(&ctx.contract_id), 0);
}

#[test]
#[should_panic]
fn test_create_trade_invalid_amount_fails() {
    let ctx = setup();
    let client = SafeTradeContractClient::new(&ctx.env, &ctx.contract_id);
    let deadline = ctx.env.ledger().timestamp() + 1_000;
    client.create_trade(
        &ctx.buyer,
        &ctx.seller,
        &ctx.token,
        &0,
        &deadline,
        &item(&ctx.env),
    );
}

#[test]
#[should_panic]
fn test_create_trade_deadline_in_past_fails() {
    let ctx = setup();
    let client = SafeTradeContractClient::new(&ctx.env, &ctx.contract_id);
    client.create_trade(
        &ctx.buyer,
        &ctx.seller,
        &ctx.token,
        &500,
        &0,
        &item(&ctx.env),
    );
}

#[test]
#[should_panic]
fn test_create_trade_same_address_fails() {
    let ctx = setup();
    let client = SafeTradeContractClient::new(&ctx.env, &ctx.contract_id);
    let deadline = ctx.env.ledger().timestamp() + 1_000;
    client.create_trade(
        &ctx.buyer,
        &ctx.buyer,
        &ctx.token,
        &500,
        &deadline,
        &item(&ctx.env),
    );
}

#[test]
#[should_panic]
fn test_confirm_receipt_wrong_caller_fails() {
    let ctx = setup();
    let client = SafeTradeContractClient::new(&ctx.env, &ctx.contract_id);
    let deadline = ctx.env.ledger().timestamp() + 1_000;
    let trade_id = client.create_trade(
        &ctx.buyer,
        &ctx.seller,
        &ctx.token,
        &500,
        &deadline,
        &item(&ctx.env),
    );
    client.confirm_receipt(&trade_id, &ctx.seller);
}

#[test]
fn test_cancel_trade_by_buyer_before_deadline() {
    let ctx = setup();
    let client = SafeTradeContractClient::new(&ctx.env, &ctx.contract_id);
    let token_client = token::Client::new(&ctx.env, &ctx.token);
    let deadline = ctx.env.ledger().timestamp() + 1_000;
    let trade_id = client.create_trade(
        &ctx.buyer,
        &ctx.seller,
        &ctx.token,
        &500,
        &deadline,
        &item(&ctx.env),
    );

    client.cancel_trade(&trade_id, &ctx.buyer);

    let trade = client.get_trade(&trade_id);
    assert_eq!(trade.status, TradeStatus::Cancelled);
    assert_eq!(token_client.balance(&ctx.buyer), 1_000_000);
}

#[test]
#[should_panic]
fn test_cancel_trade_by_seller_before_deadline_fails() {
    let ctx = setup();
    let client = SafeTradeContractClient::new(&ctx.env, &ctx.contract_id);
    let deadline = ctx.env.ledger().timestamp() + 1_000;
    let trade_id = client.create_trade(
        &ctx.buyer,
        &ctx.seller,
        &ctx.token,
        &500,
        &deadline,
        &item(&ctx.env),
    );

    client.cancel_trade(&trade_id, &ctx.seller);
}

#[test]
fn test_cancel_trade_after_deadline_by_seller() {
    let ctx = setup();
    let client = SafeTradeContractClient::new(&ctx.env, &ctx.contract_id);
    let token_client = token::Client::new(&ctx.env, &ctx.token);
    let deadline = ctx.env.ledger().timestamp() + 1_000;
    let trade_id = client.create_trade(
        &ctx.buyer,
        &ctx.seller,
        &ctx.token,
        &500,
        &deadline,
        &item(&ctx.env),
    );

    ctx.env.ledger().with_mut(|li| li.timestamp = deadline + 1);
    client.cancel_trade(&trade_id, &ctx.seller);

    let trade = client.get_trade(&trade_id);
    assert_eq!(trade.status, TradeStatus::Cancelled);
    assert_eq!(token_client.balance(&ctx.buyer), 1_000_000);
}

#[test]
fn test_open_dispute_and_resolve_dispute_seller_wins() {
    let ctx = setup();
    let client = SafeTradeContractClient::new(&ctx.env, &ctx.contract_id);
    let token_client = token::Client::new(&ctx.env, &ctx.token);
    let deadline = ctx.env.ledger().timestamp() + 1_000;
    let trade_id = client.create_trade(
        &ctx.buyer,
        &ctx.seller,
        &ctx.token,
        &500,
        &deadline,
        &item(&ctx.env),
    );

    client.open_dispute(&trade_id, &ctx.buyer);
    assert_eq!(client.get_trade(&trade_id).status, TradeStatus::Disputed);

    client.resolve_dispute(&ctx.admin, &trade_id, &ctx.seller);

    let trade = client.get_trade(&trade_id);
    assert_eq!(trade.status, TradeStatus::Completed);
    assert_eq!(token_client.balance(&ctx.seller), 500);
}

#[test]
fn test_resolve_dispute_buyer_wins() {
    let ctx = setup();
    let client = SafeTradeContractClient::new(&ctx.env, &ctx.contract_id);
    let token_client = token::Client::new(&ctx.env, &ctx.token);
    let deadline = ctx.env.ledger().timestamp() + 1_000;
    let trade_id = client.create_trade(
        &ctx.buyer,
        &ctx.seller,
        &ctx.token,
        &500,
        &deadline,
        &item(&ctx.env),
    );

    client.open_dispute(&trade_id, &ctx.buyer);
    client.resolve_dispute(&ctx.admin, &trade_id, &ctx.buyer);

    assert_eq!(token_client.balance(&ctx.buyer), 1_000_000);
}

#[test]
#[should_panic]
fn test_resolve_dispute_not_admin_fails() {
    let ctx = setup();
    let client = SafeTradeContractClient::new(&ctx.env, &ctx.contract_id);
    let deadline = ctx.env.ledger().timestamp() + 1_000;
    let trade_id = client.create_trade(
        &ctx.buyer,
        &ctx.seller,
        &ctx.token,
        &500,
        &deadline,
        &item(&ctx.env),
    );
    client.open_dispute(&trade_id, &ctx.buyer);

    let impostor = Address::generate(&ctx.env);
    client.resolve_dispute(&impostor, &trade_id, &ctx.seller);
}

#[test]
#[should_panic]
fn test_resolve_dispute_invalid_winner_fails() {
    let ctx = setup();
    let client = SafeTradeContractClient::new(&ctx.env, &ctx.contract_id);
    let deadline = ctx.env.ledger().timestamp() + 1_000;
    let trade_id = client.create_trade(
        &ctx.buyer,
        &ctx.seller,
        &ctx.token,
        &500,
        &deadline,
        &item(&ctx.env),
    );
    client.open_dispute(&trade_id, &ctx.buyer);

    let stranger = Address::generate(&ctx.env);
    client.resolve_dispute(&ctx.admin, &trade_id, &stranger);
}

#[test]
#[should_panic]
fn test_resolve_dispute_wrong_status_fails() {
    let ctx = setup();
    let client = SafeTradeContractClient::new(&ctx.env, &ctx.contract_id);
    let deadline = ctx.env.ledger().timestamp() + 1_000;
    let trade_id = client.create_trade(
        &ctx.buyer,
        &ctx.seller,
        &ctx.token,
        &500,
        &deadline,
        &item(&ctx.env),
    );

    client.resolve_dispute(&ctx.admin, &trade_id, &ctx.seller);
}

#[test]
fn test_get_trades_by_buyer_and_seller() {
    let ctx = setup();
    let client = SafeTradeContractClient::new(&ctx.env, &ctx.contract_id);
    let deadline = ctx.env.ledger().timestamp() + 1_000;

    let other_seller = Address::generate(&ctx.env);
    let other_buyer = Address::generate(&ctx.env);
    token::StellarAssetClient::new(&ctx.env, &ctx.token).mint(&other_buyer, &1_000_000);

    client.create_trade(
        &ctx.buyer,
        &ctx.seller,
        &ctx.token,
        &100,
        &deadline,
        &item(&ctx.env),
    );
    client.create_trade(
        &ctx.buyer,
        &other_seller,
        &ctx.token,
        &200,
        &deadline,
        &item(&ctx.env),
    );
    client.create_trade(
        &other_buyer,
        &ctx.seller,
        &ctx.token,
        &300,
        &deadline,
        &item(&ctx.env),
    );

    let buyer_trades = client.get_trades_by_buyer(&ctx.buyer);
    assert_eq!(buyer_trades.len(), 2);
    let mut buyer_has_100 = false;
    let mut buyer_has_200 = false;
    for t in buyer_trades.iter() {
        if t.amount == 100 {
            buyer_has_100 = true;
        }
        if t.amount == 200 {
            buyer_has_200 = true;
        }
    }
    assert!(buyer_has_100 && buyer_has_200);

    let seller_trades = client.get_trades_by_seller(&ctx.seller);
    assert_eq!(seller_trades.len(), 2);
    let mut seller_has_100 = false;
    let mut seller_has_300 = false;
    for t in seller_trades.iter() {
        if t.amount == 100 {
            seller_has_100 = true;
        }
        if t.amount == 300 {
            seller_has_300 = true;
        }
    }
    assert!(seller_has_100 && seller_has_300);
}
