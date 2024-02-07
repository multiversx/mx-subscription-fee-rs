#![allow(deprecated)]

use std::{cell::RefCell, rc::Rc};

use multiversx_sc_scenario::{
    managed_address, managed_token_id, rust_biguint, testing_framework::BlockchainStateWrapper,
    DebugApi,
};
use pair_setup::PairSetup;
use subscription_fee::pair_actions::PairActionsModule;
use subscription_setup::SubscriptionSetup;

mod pair_setup;
mod subscription_setup;

static FIRST_TOKEN_ID: &[u8] = b"MYTOKEN-123456";
static USDC_TOKEN_ID: &[u8] = b"USDC-123456";
static WEGLD_TOKEN_ID: &[u8] = b"WEGLD-123456";
static LP_TOKEN_ID: &[u8] = b"LPTOK-123456";

pub const DAILY_SUBSCRIPTION_EPOCHS: u64 = 1;

fn init_all<
    PairBuilder: 'static + Copy + Fn() -> pair::ContractObj<DebugApi>,
    SubscriptionObjBuilder: 'static + Copy + Fn() -> subscription_fee::ContractObj<DebugApi>,
>(
    pair_builder: PairBuilder,
    sub_builder: SubscriptionObjBuilder,
) -> (
    Rc<RefCell<BlockchainStateWrapper>>,
    PairSetup<PairBuilder>,
    SubscriptionSetup<SubscriptionObjBuilder>,
) {
    let mut b_mock = BlockchainStateWrapper::new();
    let owner = b_mock.create_user_account(&rust_biguint!(0));

    let b_mock_ref = RefCell::new(b_mock);
    let b_mock_rc = Rc::new(b_mock_ref);
    let payment_pair_setup = PairSetup::new(
        b_mock_rc.clone(),
        pair_builder,
        &owner,
        FIRST_TOKEN_ID,
        WEGLD_TOKEN_ID,
        LP_TOKEN_ID,
        1_000_000_000,
        1_000_000_000,
    );
    let stable_pair_setup = PairSetup::new(
        b_mock_rc.clone(),
        pair_builder,
        &owner,
        WEGLD_TOKEN_ID,
        USDC_TOKEN_ID,
        LP_TOKEN_ID,
        1_000_000_000,
        2_000_000_000,
    );
    let sub_sc = SubscriptionSetup::new(
        b_mock_rc.clone(),
        sub_builder,
        &owner,
        stable_pair_setup.pair_wrapper.address_ref(),
        vec![FIRST_TOKEN_ID.to_vec()],
    );

    b_mock_rc
        .borrow_mut()
        .execute_tx(&owner, &sub_sc.s_wrapper, &rust_biguint!(0), |sc| {
            sc.add_pair_address(
                managed_token_id!(FIRST_TOKEN_ID),
                managed_address!(payment_pair_setup.pair_wrapper.address_ref()),
            );
        })
        .assert_ok();

    b_mock_rc
        .borrow_mut()
        .execute_tx(&owner, &sub_sc.s_wrapper, &rust_biguint!(0), |sc| {
            sc.add_pair_address(
                managed_token_id!(WEGLD_TOKEN_ID),
                managed_address!(stable_pair_setup.pair_wrapper.address_ref()),
            );
        })
        .assert_ok();

    (b_mock_rc, payment_pair_setup, sub_sc)
}

#[test]
fn init_test() {
    let _ = init_all(pair::contract_obj, subscription_fee::contract_obj);
}

#[test]
fn register_service_test() {
    let (b_mock_rc, _pair_setup, mut sub_sc) =
        init_all(pair::contract_obj, subscription_fee::contract_obj);
    let rust_zero = rust_biguint!(0);

    let rand_service = b_mock_rc.borrow_mut().create_user_account(&rust_zero);
    sub_sc
        .call_register_service(
            &rand_service,
            vec![(
                Some(FIRST_TOKEN_ID.to_vec()),
                1_000,
                false,
                DAILY_SUBSCRIPTION_EPOCHS,
            )],
        )
        .assert_ok();
}

#[test]
fn approve_test() {
    let (b_mock_rc, _pair_setup, mut sub_sc) =
        init_all(pair::contract_obj, subscription_fee::contract_obj);
    let rust_zero = rust_biguint!(0);

    let rand_service = b_mock_rc.borrow_mut().create_user_account(&rust_zero);
    sub_sc
        .call_register_service(
            &rand_service,
            vec![(
                Some(FIRST_TOKEN_ID.to_vec()),
                1_000,
                false,
                DAILY_SUBSCRIPTION_EPOCHS,
            )],
        )
        .assert_ok();

    sub_sc.call_approve_service(&rand_service).assert_ok();
}

#[test]
fn unregister_service_test() {
    let (b_mock_rc, _pair_setup, mut sub_sc) =
        init_all(pair::contract_obj, subscription_fee::contract_obj);
    let rust_zero = rust_biguint!(0);

    let rand_service = b_mock_rc.borrow_mut().create_user_account(&rust_zero);
    sub_sc
        .call_register_service(
            &rand_service,
            vec![(
                Some(FIRST_TOKEN_ID.to_vec()),
                1_000,
                false,
                DAILY_SUBSCRIPTION_EPOCHS,
            )],
        )
        .assert_ok();

    sub_sc.call_approve_service(&rand_service).assert_ok();
    sub_sc.call_unregister_service(&rand_service).assert_ok();
}

#[test]
fn subscribe_before_deposit_test() {
    let (b_mock_rc, _pair_setup, mut sub_sc) =
        init_all(pair::contract_obj, subscription_fee::contract_obj);
    let rust_zero = rust_biguint!(0);

    let rand_service = b_mock_rc.borrow_mut().create_user_account(&rust_zero);
    sub_sc
        .call_register_service(
            &rand_service,
            vec![(
                Some(FIRST_TOKEN_ID.to_vec()),
                1_000,
                false,
                DAILY_SUBSCRIPTION_EPOCHS,
            )],
        )
        .assert_ok();

    let user = b_mock_rc.borrow_mut().create_user_account(&rust_zero);
    sub_sc
        .call_subscribe(&user, vec![(1, 0)])
        .assert_user_error("Unknown address");
}

#[test]
fn subscribe_before_approve_test() {
    let (b_mock_rc, _pair_setup, mut sub_sc) =
        init_all(pair::contract_obj, subscription_fee::contract_obj);
    let rust_zero = rust_biguint!(0);

    let rand_service = b_mock_rc.borrow_mut().create_user_account(&rust_zero);
    sub_sc
        .call_register_service(
            &rand_service,
            vec![(
                Some(FIRST_TOKEN_ID.to_vec()),
                1_000,
                false,
                DAILY_SUBSCRIPTION_EPOCHS,
            )],
        )
        .assert_ok();

    let user = b_mock_rc.borrow_mut().create_user_account(&rust_zero);
    b_mock_rc
        .borrow_mut()
        .set_esdt_balance(&user, FIRST_TOKEN_ID, &rust_biguint!(1_000_000));

    sub_sc
        .call_deposit(&user, FIRST_TOKEN_ID, 1_000_000)
        .assert_ok();

    sub_sc
        .call_subscribe(&user, vec![(1, 0)])
        .assert_user_error("Invalid service index");
}

#[test]
fn subscribe_ok_test() {
    let (b_mock_rc, _pair_setup, mut sub_sc) =
        init_all(pair::contract_obj, subscription_fee::contract_obj);
    let rust_zero = rust_biguint!(0);

    let rand_service = b_mock_rc.borrow_mut().create_user_account(&rust_zero);
    sub_sc
        .call_register_service(
            &rand_service,
            vec![(
                Some(FIRST_TOKEN_ID.to_vec()),
                1_000,
                false,
                DAILY_SUBSCRIPTION_EPOCHS,
            )],
        )
        .assert_ok();

    sub_sc.call_approve_service(&rand_service).assert_ok();

    let user = b_mock_rc.borrow_mut().create_user_account(&rust_zero);
    b_mock_rc
        .borrow_mut()
        .set_esdt_balance(&user, FIRST_TOKEN_ID, &rust_biguint!(1_000_000));

    sub_sc
        .call_deposit(&user, FIRST_TOKEN_ID, 1_000_000)
        .assert_ok();

    sub_sc.call_subscribe(&user, vec![(1, 0)]).assert_ok();
}

#[test]
fn subtract_ok_test() {
    let (b_mock_rc, _pair_setup, mut sub_sc) =
        init_all(pair::contract_obj, subscription_fee::contract_obj);
    let rust_zero = rust_biguint!(0);

    let rand_service = b_mock_rc.borrow_mut().create_user_account(&rust_zero);
    sub_sc
        .call_register_service(
            &rand_service,
            vec![(
                Some(FIRST_TOKEN_ID.to_vec()),
                1_000,
                false,
                DAILY_SUBSCRIPTION_EPOCHS,
            )],
        )
        .assert_ok();

    sub_sc.call_approve_service(&rand_service).assert_ok();

    let user = b_mock_rc.borrow_mut().create_user_account(&rust_zero);
    b_mock_rc
        .borrow_mut()
        .set_esdt_balance(&user, FIRST_TOKEN_ID, &rust_biguint!(1_000_000));

    sub_sc
        .call_deposit(&user, FIRST_TOKEN_ID, 1_000_000)
        .assert_ok();

    sub_sc.call_subscribe(&user, vec![(1, 0)]).assert_ok();

    b_mock_rc.borrow_mut().set_block_epoch(10);

    sub_sc
        .call_subtract_payment(&rand_service, 0, 1)
        .assert_ok();

    b_mock_rc
        .borrow()
        .check_esdt_balance(&rand_service, FIRST_TOKEN_ID, &rust_biguint!(1_000));
}

#[test]
fn try_subtract_twice_same_day() {
    let (b_mock_rc, _pair_setup, mut sub_sc) =
        init_all(pair::contract_obj, subscription_fee::contract_obj);
    let rust_zero = rust_biguint!(0);

    let rand_service = b_mock_rc.borrow_mut().create_user_account(&rust_zero);
    sub_sc
        .call_register_service(
            &rand_service,
            vec![(
                Some(FIRST_TOKEN_ID.to_vec()),
                1_000,
                false,
                DAILY_SUBSCRIPTION_EPOCHS,
            )],
        )
        .assert_ok();

    sub_sc.call_approve_service(&rand_service).assert_ok();

    let user = b_mock_rc.borrow_mut().create_user_account(&rust_zero);
    b_mock_rc
        .borrow_mut()
        .set_esdt_balance(&user, FIRST_TOKEN_ID, &rust_biguint!(1_000_000));

    sub_sc
        .call_deposit(&user, FIRST_TOKEN_ID, 1_000_000)
        .assert_ok();

    sub_sc.call_subscribe(&user, vec![(1, 0)]).assert_ok();

    b_mock_rc.borrow_mut().set_block_epoch(10);

    sub_sc
        .call_subtract_payment(&rand_service, 0, 1)
        .assert_ok();

    b_mock_rc
        .borrow()
        .check_esdt_balance(&rand_service, FIRST_TOKEN_ID, &rust_biguint!(1_000));

    sub_sc
        .call_subtract_payment(&rand_service, 0, 1)
        .assert_user_error("Cannot subtract yet");

    // still same balance
    b_mock_rc
        .borrow()
        .check_esdt_balance(&rand_service, FIRST_TOKEN_ID, &rust_biguint!(1_000));
}

#[test]
fn any_token_subtract_fee_test() {
    let (b_mock_rc, _pair_setup, mut sub_sc) =
        init_all(pair::contract_obj, subscription_fee::contract_obj);
    let rust_zero = rust_biguint!(0);

    let rand_service = b_mock_rc.borrow_mut().create_user_account(&rust_zero);
    sub_sc
        .call_register_service(
            &rand_service,
            vec![(None, 1_000, false, DAILY_SUBSCRIPTION_EPOCHS)],
        )
        .assert_ok();

    sub_sc.call_approve_service(&rand_service).assert_ok();

    let user = b_mock_rc.borrow_mut().create_user_account(&rust_zero);
    b_mock_rc
        .borrow_mut()
        .set_esdt_balance(&user, FIRST_TOKEN_ID, &rust_biguint!(1_000_000));

    sub_sc
        .call_deposit(&user, FIRST_TOKEN_ID, 1_000_000)
        .assert_ok();

    sub_sc.call_subscribe(&user, vec![(1, 0)]).assert_ok();

    b_mock_rc.borrow_mut().set_block_epoch(10);

    sub_sc
        .call_subtract_payment(&rand_service, 0, 1)
        .assert_ok();

    // pair has 1:2 token ratio, so to cover the 1000 USDC price, the payment consists of 500 deposited tokens
    b_mock_rc
        .borrow()
        .check_esdt_balance(&rand_service, FIRST_TOKEN_ID, &rust_biguint!(500));
}

#[test]
fn withdraw_tokens_test() {
    let (b_mock_rc, _pair_setup, mut sub_sc) =
        init_all(pair::contract_obj, subscription_fee::contract_obj);
    let rust_zero = rust_biguint!(0);

    let rand_service = b_mock_rc.borrow_mut().create_user_account(&rust_zero);
    sub_sc
        .call_register_service(
            &rand_service,
            vec![(
                Some(FIRST_TOKEN_ID.to_vec()),
                1_000,
                false,
                DAILY_SUBSCRIPTION_EPOCHS,
            )],
        )
        .assert_ok();

    sub_sc.call_approve_service(&rand_service).assert_ok();

    let user = b_mock_rc.borrow_mut().create_user_account(&rust_zero);
    b_mock_rc
        .borrow_mut()
        .set_esdt_balance(&user, FIRST_TOKEN_ID, &rust_biguint!(1_000_000));

    sub_sc
        .call_deposit(&user, FIRST_TOKEN_ID, 1_000_000)
        .assert_ok();

    sub_sc.call_subscribe(&user, vec![(1, 0)]).assert_ok();

    b_mock_rc.borrow_mut().set_block_epoch(10);

    sub_sc
        .call_withdraw_funds(&user, vec![(FIRST_TOKEN_ID.to_vec(), 999_999)])
        .assert_ok();

    b_mock_rc
        .borrow()
        .check_esdt_balance(&user, FIRST_TOKEN_ID, &rust_biguint!(999_999));

    sub_sc
        .call_subtract_payment(&rand_service, 0, 1)
        .assert_ok();

    // not enough to subtract
    b_mock_rc
        .borrow()
        .check_esdt_balance(&rand_service, FIRST_TOKEN_ID, &rust_biguint!(0));
}

#[test]
fn try_substract_from_unsubscribed_user_test() {
    let (b_mock_rc, _pair_setup, mut sub_sc) =
        init_all(pair::contract_obj, subscription_fee::contract_obj);
    let rust_zero = rust_biguint!(0);

    // Create service with 2 service indexes
    let rand_service = b_mock_rc.borrow_mut().create_user_account(&rust_zero);
    sub_sc
        .call_register_service(
            &rand_service,
            vec![
                (
                    Some(FIRST_TOKEN_ID.to_vec()),
                    1_000,
                    false,
                    DAILY_SUBSCRIPTION_EPOCHS,
                ),
                (
                    Some(FIRST_TOKEN_ID.to_vec()),
                    500,
                    false,
                    DAILY_SUBSCRIPTION_EPOCHS,
                ),
            ],
        )
        .assert_ok();

    sub_sc.call_approve_service(&rand_service).assert_ok();

    let user = b_mock_rc.borrow_mut().create_user_account(&rust_zero);
    b_mock_rc
        .borrow_mut()
        .set_esdt_balance(&user, FIRST_TOKEN_ID, &rust_biguint!(1_000_000));

    sub_sc
        .call_deposit(&user, FIRST_TOKEN_ID, 1_000_000)
        .assert_ok();

    // User subscribes to the second service index
    sub_sc.call_subscribe(&user, vec![(1, 1)]).assert_ok();

    b_mock_rc.borrow_mut().set_block_epoch(10);

    // The endpoint returns a ScResult::Err(()), but does not stop the execution
    sub_sc
        .call_subtract_payment_with_sc_error(&rand_service, 0, 1)
        .assert_ok();

    // The service was not able to substract the payment for the wrong service index
    b_mock_rc
        .borrow()
        .check_esdt_balance(&rand_service, FIRST_TOKEN_ID, &rust_biguint!(0));

    // The user pays the fee for the correct service index
    sub_sc
        .call_subtract_payment(&rand_service, 1, 1)
        .assert_ok();
    // The service has sucessfully deducted the amount for the correct service index
    b_mock_rc
        .borrow()
        .check_esdt_balance(&rand_service, FIRST_TOKEN_ID, &rust_biguint!(500));
}
