#![allow(deprecated)]

use std::{cell::RefCell, rc::Rc};

use multiversx_sc_scenario::{
    managed_address, managed_token_id, rust_biguint, testing_framework::BlockchainStateWrapper,
    DebugApi,
};
use pair_setup::PairSetup;
use subscriber_setup::SubscriberSetup;
use subscription_fee::{pair_actions::PairActionsModule, service::SubscriptionType};
use subscription_setup::SubscriptionSetup;

mod pair_setup;
mod subscriber_setup;
mod subscription_setup;

static FIRST_TOKEN_ID: &[u8] = b"MYTOKEN-123456";
static USDC_TOKEN_ID: &[u8] = b"USDC-123456";
static LP_TOKEN_ID: &[u8] = b"LPTOK-123456";

fn init_all<
    PairBuilder: 'static + Copy + Fn() -> pair::ContractObj<DebugApi>,
    SubscriptionObjBuilder: 'static + Copy + Fn() -> subscription_fee::ContractObj<DebugApi>,
    SubscriberBuilder: 'static + Copy + Fn() -> subscriber::ContractObj<DebugApi>,
>(
    pair_builder: PairBuilder,
    sub_builder: SubscriptionObjBuilder,
    subscriber_builder: SubscriberBuilder,
) -> (
    Rc<RefCell<BlockchainStateWrapper>>,
    PairSetup<PairBuilder>,
    SubscriptionSetup<SubscriptionObjBuilder>,
    SubscriberSetup<SubscriberBuilder>,
) {
    let mut b_mock = BlockchainStateWrapper::new();
    let owner = b_mock.create_user_account(&rust_biguint!(0));

    let b_mock_ref = RefCell::new(b_mock);
    let b_mock_rc = Rc::new(b_mock_ref);
    let pair_setup = PairSetup::new(
        b_mock_rc.clone(),
        pair_builder,
        &owner,
        FIRST_TOKEN_ID,
        USDC_TOKEN_ID,
        LP_TOKEN_ID,
        1_000_000_000,
        2_000_000_000,
    );
    let sub_sc = SubscriptionSetup::new(
        b_mock_rc.clone(),
        sub_builder,
        &owner,
        pair_setup.pair_wrapper.address_ref(),
        vec![FIRST_TOKEN_ID.to_vec()],
    );

    b_mock_rc
        .borrow_mut()
        .execute_tx(&owner, &sub_sc.s_wrapper, &rust_biguint!(0), |sc| {
            sc.add_usdc_pair(
                managed_token_id!(FIRST_TOKEN_ID),
                managed_address!(pair_setup.pair_wrapper.address_ref()),
            );
        })
        .assert_ok();

    let subscriber = SubscriberSetup::new(
        b_mock_rc.clone(),
        subscriber_builder,
        sub_sc.s_wrapper.address_ref(),
        &owner,
    );

    (b_mock_rc, pair_setup, sub_sc, subscriber)
}

#[test]
fn init_test() {
    let _ = init_all(
        pair::contract_obj,
        subscription_fee::contract_obj,
        subscriber::contract_obj,
    );
}

#[test]
fn register_service_test() {
    let (_, _, mut subscription_setup, mut subscriber_setup) = init_all(
        pair::contract_obj,
        subscription_fee::contract_obj,
        subscriber::contract_obj,
    );

    subscriber_setup
        .call_register_service(vec![
            (
                subscriber_setup.sub_wrapper.address_ref().clone(),
                Some(FIRST_TOKEN_ID.to_vec()),
                1_000,
            ),
            (
                subscriber_setup.sub_wrapper.address_ref().clone(),
                Some(FIRST_TOKEN_ID.to_vec()),
                500,
            ),
        ])
        .assert_ok();

    subscription_setup
        .call_approve_service(subscriber_setup.sub_wrapper.address_ref())
        .assert_ok();
}

#[test]
fn unregister_test() {
    let (_, _, mut subscription_setup, mut subscriber_setup) = init_all(
        pair::contract_obj,
        subscription_fee::contract_obj,
        subscriber::contract_obj,
    );

    subscriber_setup
        .call_register_service(vec![
            (
                subscriber_setup.sub_wrapper.address_ref().clone(),
                Some(FIRST_TOKEN_ID.to_vec()),
                1_000,
            ),
            (
                subscriber_setup.sub_wrapper.address_ref().clone(),
                Some(FIRST_TOKEN_ID.to_vec()),
                500,
            ),
        ])
        .assert_ok();

    subscription_setup
        .call_approve_service(subscriber_setup.sub_wrapper.address_ref())
        .assert_ok();

    subscriber_setup.call_unregister_service().assert_ok();
}

#[test]
fn try_subscribe_after_unregister() {
    let (b_mock_rc, _, mut subscription_setup, mut subscriber_setup) = init_all(
        pair::contract_obj,
        subscription_fee::contract_obj,
        subscriber::contract_obj,
    );

    subscriber_setup
        .call_register_service(vec![
            (
                subscriber_setup.sub_wrapper.address_ref().clone(),
                Some(FIRST_TOKEN_ID.to_vec()),
                1_000,
            ),
            (
                subscriber_setup.sub_wrapper.address_ref().clone(),
                Some(FIRST_TOKEN_ID.to_vec()),
                500,
            ),
        ])
        .assert_ok();

    subscription_setup
        .call_approve_service(subscriber_setup.sub_wrapper.address_ref())
        .assert_ok();

    subscriber_setup.call_unregister_service().assert_ok();

    let user = b_mock_rc
        .borrow_mut()
        .create_user_account(&rust_biguint!(0));
    b_mock_rc
        .borrow_mut()
        .set_esdt_balance(&user, FIRST_TOKEN_ID, &rust_biguint!(1_000_000));

    subscription_setup
        .call_deposit(&user, FIRST_TOKEN_ID, 1_000_000)
        .assert_ok();

    subscription_setup
        .call_subscribe(&user, vec![(1, 0, SubscriptionType::Daily)])
        .assert_user_error("Invalid service index");
}

#[test]
fn subscribe_ok_test() {
    let (b_mock_rc, _, mut subscription_setup, mut subscriber_setup) = init_all(
        pair::contract_obj,
        subscription_fee::contract_obj,
        subscriber::contract_obj,
    );

    subscriber_setup
        .call_register_service(vec![
            (
                subscriber_setup.sub_wrapper.address_ref().clone(),
                Some(FIRST_TOKEN_ID.to_vec()),
                1_000,
            ),
            (
                subscriber_setup.sub_wrapper.address_ref().clone(),
                Some(FIRST_TOKEN_ID.to_vec()),
                500,
            ),
        ])
        .assert_ok();

    subscription_setup
        .call_approve_service(subscriber_setup.sub_wrapper.address_ref())
        .assert_ok();

    let user = b_mock_rc
        .borrow_mut()
        .create_user_account(&rust_biguint!(0));
    b_mock_rc
        .borrow_mut()
        .set_esdt_balance(&user, FIRST_TOKEN_ID, &rust_biguint!(1_000_000));

    subscription_setup
        .call_deposit(&user, FIRST_TOKEN_ID, 1_000_000)
        .assert_ok();

    subscription_setup
        .call_subscribe(&user, vec![(1, 0, SubscriptionType::Daily)])
        .assert_ok();
}

#[test]
fn perform_daily_action_test() {
    let (b_mock_rc, _, mut subscription_setup, mut subscriber_setup) = init_all(
        pair::contract_obj,
        subscription_fee::contract_obj,
        subscriber::contract_obj,
    );

    subscriber_setup
        .call_register_service(vec![
            (
                subscriber_setup.sub_wrapper.address_ref().clone(),
                Some(FIRST_TOKEN_ID.to_vec()),
                1_000,
            ),
            (
                subscriber_setup.sub_wrapper.address_ref().clone(),
                Some(FIRST_TOKEN_ID.to_vec()),
                500,
            ),
        ])
        .assert_ok();

    subscription_setup
        .call_approve_service(subscriber_setup.sub_wrapper.address_ref())
        .assert_ok();

    let user = b_mock_rc
        .borrow_mut()
        .create_user_account(&rust_biguint!(0));
    b_mock_rc
        .borrow_mut()
        .set_esdt_balance(&user, FIRST_TOKEN_ID, &rust_biguint!(1_000_000));

    subscription_setup
        .call_deposit(&user, FIRST_TOKEN_ID, 1_000_000)
        .assert_ok();

    subscription_setup
        .call_subscribe(&user, vec![(1, 0, SubscriptionType::Daily)])
        .assert_ok();

    b_mock_rc.borrow_mut().set_block_epoch(10);

    subscriber_setup.call_subtract_payment(0).assert_ok();
    subscriber_setup.call_perform_action(0).assert_ok();

    b_mock_rc.borrow().check_esdt_balance(
        subscriber_setup.sub_wrapper.address_ref(),
        FIRST_TOKEN_ID,
        &rust_biguint!(1_000 * 30),
    );

    // try perform operation again, same epoch
    subscriber_setup.call_perform_action(0).assert_ok();

    // still same balance, no funds subtracted
    b_mock_rc.borrow().check_esdt_balance(
        subscriber_setup.sub_wrapper.address_ref(),
        FIRST_TOKEN_ID,
        &rust_biguint!(1_000 * 30),
    );

    b_mock_rc.borrow_mut().set_block_epoch(11);

    subscriber_setup.call_perform_action(0).assert_ok();

    // still same balance, subtraction is done manually once per month
    b_mock_rc.borrow().check_esdt_balance(
        subscriber_setup.sub_wrapper.address_ref(),
        FIRST_TOKEN_ID,
        &rust_biguint!(1_000 * 30),
    );
}
