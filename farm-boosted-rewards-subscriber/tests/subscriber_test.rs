#![allow(deprecated)]

use std::{cell::RefCell, rc::Rc};

use farm_setup::FarmSetup;
use multiversx_sc_scenario::{
    managed_address, managed_token_id, rust_biguint, testing_framework::BlockchainStateWrapper,
    DebugApi,
};
use pair_setup::PairSetup;
use simple_lock::locked_token::LockedTokenAttributes;
use subscriber_setup::SubscriberSetup;
use subscription_fee::{pair_actions::PairActionsModule, service::SubscriptionType};
use subscription_setup::SubscriptionSetup;

mod farm_setup;
mod pair_setup;
mod subscriber_setup;
mod subscription_setup;

static FIRST_TOKEN_ID: &[u8] = b"MYTOKEN-123456";
static USDC_TOKEN_ID: &[u8] = b"USDC-123456";
static LP_TOKEN_ID: &[u8] = b"LPTOK-123456";
static REWARD_TOKEN_ID: &[u8] = b"MEX-123456";
static LOCKED_TOKEN_ID: &[u8] = b"XMEX-123456";

#[allow(type_complexity)]
fn init_all<
    PairBuilder: 'static + Copy + Fn() -> pair::ContractObj<DebugApi>,
    FarmBuilder: 'static + Copy + Fn() -> farm_with_locked_rewards::ContractObj<DebugApi>,
    EnergyFactoryBuilder: 'static + Copy + Fn() -> energy_factory::ContractObj<DebugApi>,
    SubscriptionObjBuilder: 'static + Copy + Fn() -> subscription_fee::ContractObj<DebugApi>,
    SubscriberBuilder: 'static + Copy + Fn() -> farm_boosted_rewards_subscriber::ContractObj<DebugApi>,
>(
    pair_builder: PairBuilder,
    farm_builder: FarmBuilder,
    energy_factory_builder: EnergyFactoryBuilder,
    sub_builder: SubscriptionObjBuilder,
    subscriber_builder: SubscriberBuilder,
) -> (
    Rc<RefCell<BlockchainStateWrapper>>,
    PairSetup<PairBuilder>,
    FarmSetup<FarmBuilder, EnergyFactoryBuilder>,
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

    let farm_setup = FarmSetup::new(
        b_mock_rc.clone(),
        &owner,
        REWARD_TOKEN_ID,
        LOCKED_TOKEN_ID,
        LP_TOKEN_ID,
        pair_setup.pair_wrapper.address_ref(),
        farm_builder,
        energy_factory_builder,
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
        REWARD_TOKEN_ID,
    );

    (b_mock_rc, pair_setup, farm_setup, sub_sc, subscriber)
}

#[test]
fn init_test() {
    let _ = init_all(
        pair::contract_obj,
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
        subscription_fee::contract_obj,
        farm_boosted_rewards_subscriber::contract_obj,
    );
}

#[test]
fn claim_boosted_rewards_for_user_test() {
    let (b_mock_rc, _pair_setup, mut farm_setup, mut subscription_setup, mut subscriber_setup) =
        init_all(
            pair::contract_obj,
            farm_with_locked_rewards::contract_obj,
            energy_factory::contract_obj,
            subscription_fee::contract_obj,
            farm_boosted_rewards_subscriber::contract_obj,
        );

    let user = b_mock_rc
        .borrow_mut()
        .create_user_account(&rust_biguint!(0));
    let user_id = 1;
    b_mock_rc.borrow_mut().set_block_epoch(2);

    subscriber_setup
        .call_register_service(vec![
            (
                farm_setup.farm_wrapper.address_ref().clone(),
                Some(FIRST_TOKEN_ID.to_vec()),
                1_000,
            ),
            (
                farm_setup.farm_wrapper.address_ref().clone(),
                Some(FIRST_TOKEN_ID.to_vec()),
                500,
            ),
        ])
        .assert_ok();

    subscription_setup
        .call_approve_service(subscriber_setup.sub_wrapper.address_ref())
        .assert_ok();

    b_mock_rc
        .borrow_mut()
        .set_esdt_balance(&user, FIRST_TOKEN_ID, &rust_biguint!(1_000_000));

    subscription_setup
        .call_deposit(&user, FIRST_TOKEN_ID, 1_000_000)
        .assert_ok();

    subscription_setup
        .call_subscribe(&user, vec![(1, 0, SubscriptionType::Daily)])
        .assert_ok();

    // Generate farm rewards
    let user_token_amount = 100_000_000;
    b_mock_rc
        .borrow_mut()
        .set_esdt_balance(&user, LP_TOKEN_ID, &rust_biguint!(user_token_amount));
    farm_setup.set_user_energy(&user, 1_000, 2, 1);
    farm_setup.enter_farm(&user, LP_TOKEN_ID, user_token_amount);
    let _ = farm_setup.claim_rewards(&user, 1, user_token_amount);
    b_mock_rc.borrow_mut().set_block_nonce(10);
    b_mock_rc.borrow_mut().set_block_epoch(6);
    farm_setup.set_user_energy(&user, 1_000, 6, 1);
    farm_setup.claim_rewards(&user, 2, user_token_amount);

    let base_rewards = 7_500;
    b_mock_rc
        .borrow()
        .check_nft_balance::<LockedTokenAttributes<DebugApi>>(
            &user,
            LOCKED_TOKEN_ID,
            1,
            &rust_biguint!(base_rewards),
            None,
        );

    b_mock_rc.borrow_mut().set_block_epoch(10);
    farm_setup.set_user_energy(&user, 1_000, 10, 1);

    farm_setup
        .call_allow_external_claim_boosted_rewards(&user, true)
        .assert_ok();

    subscriber_setup.call_subtract_payment(0).assert_ok();
    subscriber_setup.call_perform_action(0, user_id).assert_ok();

    // Check that the subscriber claimed the boosted amount for the user
    let boosted_rewards = 2_500;
    b_mock_rc
        .borrow()
        .check_nft_balance::<LockedTokenAttributes<DebugApi>>(
            &user,
            LOCKED_TOKEN_ID,
            1,
            &rust_biguint!(base_rewards + boosted_rewards), // base rewards are minted with the same nonce
            None,
        );

    b_mock_rc.borrow().check_esdt_balance(
        subscriber_setup.sub_wrapper.address_ref(),
        FIRST_TOKEN_ID,
        &rust_biguint!(1_000 * 30),
    );

    // try perform operation again, same epoch
    subscriber_setup.call_perform_action(0, user_id).assert_ok();

    // user has the same balance
    b_mock_rc
        .borrow()
        .check_nft_balance::<LockedTokenAttributes<DebugApi>>(
            &user,
            LOCKED_TOKEN_ID,
            1,
            &rust_biguint!(base_rewards + boosted_rewards), // base rewards are minted with the same nonce
            None,
        );

    // still same subscriber balance, no funds subtracted
    b_mock_rc.borrow().check_esdt_balance(
        subscriber_setup.sub_wrapper.address_ref(),
        FIRST_TOKEN_ID,
        &rust_biguint!(1_000 * 30),
    );

    b_mock_rc.borrow_mut().set_block_epoch(11);

    subscriber_setup.call_perform_action(0, user_id).assert_ok();

    // still same balance, subtraction is done manually once per month
    b_mock_rc.borrow().check_esdt_balance(
        subscriber_setup.sub_wrapper.address_ref(),
        FIRST_TOKEN_ID,
        &rust_biguint!(1_000 * 30),
    );
}

// TODO - implement the following tests
#[test]
fn claim_boosted_rewards_for_user_multiple_farms_test() {}

#[test]
fn claim_boosted_rewards_for_premium_user_test() {}
