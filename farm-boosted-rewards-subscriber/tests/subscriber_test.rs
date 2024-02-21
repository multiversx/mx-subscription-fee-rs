#![allow(deprecated)]

use std::{cell::RefCell, rc::Rc};

use farm_boosted_rewards_subscriber::subscriber_config::SubscriberConfigModule;
use farm_setup::FarmSetup;
use multiversx_sc_scenario::{
    managed_address, managed_biguint, managed_token_id, rust_biguint,
    testing_framework::BlockchainStateWrapper, DebugApi,
};
use pair_setup::PairSetup;
use simple_lock::locked_token::LockedTokenAttributes;
use subscriber_setup::SubscriberSetup;
use subscription_fee::pair_actions::PairActionsModule;
use subscription_setup::SubscriptionSetup;

mod farm_setup;
mod pair_setup;
mod subscriber_setup;
mod subscription_setup;

static USDC_TOKEN_ID: &[u8] = b"USDC-123456";
static WEGLD_TOKEN_ID: &[u8] = b"WEGLD-123456";
static LP_TOKEN_ID: &[u8] = b"LPTOK-123456";
static STABLE_LP_TOKEN_ID: &[u8] = b"SLPTOK-123456";
static REWARD_TOKEN_ID: &[u8] = b"MEX-123456";
static LOCKED_TOKEN_ID: &[u8] = b"XMEX-123456";
const DEFAULT_BOOSTED_YIELDS_PERCENTAGE: u64 = 2_500; // 25%
pub const WEEKLY_SUBSCRIPTION_EPOCHS: u64 = 7;

pub const STANDARD_SERVICE: usize = 0;
pub const PREMIUM_SERVICE: usize = 1;

#[allow(clippy::type_complexity)]
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
    PairSetup<PairBuilder>,
    FarmSetup<FarmBuilder, EnergyFactoryBuilder>,
    SubscriptionSetup<SubscriptionObjBuilder>,
    SubscriberSetup<SubscriberBuilder>,
) {
    let mut b_mock = BlockchainStateWrapper::new();
    let owner = b_mock.create_user_account(&rust_biguint!(0));

    let b_mock_ref = RefCell::new(b_mock);
    let b_mock_rc = Rc::new(b_mock_ref);
    let mex_pair_setup = PairSetup::new(
        b_mock_rc.clone(),
        pair_builder,
        &owner,
        WEGLD_TOKEN_ID,
        REWARD_TOKEN_ID,
        LP_TOKEN_ID,
        1_000_000_000,
        2_000_000_000,
    );
    let stable_pair_setup = PairSetup::new(
        b_mock_rc.clone(),
        pair_builder,
        &owner,
        WEGLD_TOKEN_ID,
        USDC_TOKEN_ID,
        STABLE_LP_TOKEN_ID,
        1_000_000_000,
        2_000_000_000,
    );

    let farm_setup = FarmSetup::new(
        b_mock_rc.clone(),
        &owner,
        REWARD_TOKEN_ID,
        LOCKED_TOKEN_ID,
        LP_TOKEN_ID,
        DEFAULT_BOOSTED_YIELDS_PERCENTAGE,
        mex_pair_setup.pair_wrapper.address_ref(),
        farm_builder,
        energy_factory_builder,
    );

    let sub_sc = SubscriptionSetup::new(
        b_mock_rc.clone(),
        sub_builder,
        &owner,
        stable_pair_setup.pair_wrapper.address_ref(),
        vec![WEGLD_TOKEN_ID.to_vec()],
    );

    b_mock_rc
        .borrow_mut()
        .execute_tx(&owner, &sub_sc.s_wrapper, &rust_biguint!(0), |sc| {
            sc.add_pair_address(
                managed_token_id!(WEGLD_TOKEN_ID),
                managed_address!(stable_pair_setup.pair_wrapper.address_ref()),
            );
        })
        .assert_ok();

    let subscriber = SubscriberSetup::new(
        b_mock_rc.clone(),
        subscriber_builder,
        sub_sc.s_wrapper.address_ref(),
        mex_pair_setup.pair_wrapper.address_ref(),
        farm_setup.energy_factory_wrapper.address_ref(),
        &owner,
        REWARD_TOKEN_ID,
        WEGLD_TOKEN_ID,
    );

    (
        b_mock_rc,
        mex_pair_setup,
        stable_pair_setup,
        farm_setup,
        sub_sc,
        subscriber,
    )
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
    let (
        b_mock_rc,
        _mex_pair_setup,
        _stable_pair_setup,
        mut farm_setup,
        mut subscription_setup,
        mut subscriber_setup,
    ) = init_all(
        pair::contract_obj,
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
        subscription_fee::contract_obj,
        farm_boosted_rewards_subscriber::contract_obj,
    );

    let farm_id = subscriber_setup.call_add_farm(farm_setup.farm_wrapper.address_ref());
    let farm_list = vec![farm_id];

    let user = b_mock_rc
        .borrow_mut()
        .create_user_account(&rust_biguint!(0));
    let user_id = 1;
    b_mock_rc.borrow_mut().set_block_epoch(2);

    subscriber_setup
        .call_register_service(vec![
            (
                Some(WEGLD_TOKEN_ID.to_vec()),
                1_000,
                false,
                WEEKLY_SUBSCRIPTION_EPOCHS,
            ),
            (
                Some(WEGLD_TOKEN_ID.to_vec()),
                500,
                false,
                WEEKLY_SUBSCRIPTION_EPOCHS,
            ),
        ])
        .assert_ok();

    subscription_setup
        .call_approve_service(subscriber_setup.sub_wrapper.address_ref())
        .assert_ok();

    b_mock_rc
        .borrow_mut()
        .set_esdt_balance(&user, WEGLD_TOKEN_ID, &rust_biguint!(1_000_000));

    subscription_setup
        .call_deposit(&user, WEGLD_TOKEN_ID, 1_000_000)
        .assert_ok();

    subscription_setup
        .call_subscribe(&user, vec![(1, STANDARD_SERVICE), (1, PREMIUM_SERVICE)])
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

    // Set user energy below the threshold
    b_mock_rc.borrow_mut().set_block_epoch(10);
    farm_setup.set_user_energy(&user, 900, 10, 1);

    farm_setup
        .call_allow_external_claim_boosted_rewards(&user, true)
        .assert_ok();

    subscriber_setup
        .call_subtract_payment(vec![user_id])
        .assert_ok();
    subscriber_setup
        .call_perform_claim_boosted(user_id, farm_list.clone())
        .assert_ok();

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
        WEGLD_TOKEN_ID,
        &rust_biguint!(1_000),
    );

    // try perform operation again, same epoch
    subscriber_setup
        .call_perform_claim_boosted(user_id, farm_list.clone())
        .assert_ok();

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
        WEGLD_TOKEN_ID,
        &rust_biguint!(1_000),
    );

    b_mock_rc.borrow_mut().set_block_epoch(11);

    subscriber_setup
        .call_perform_claim_boosted(user_id, farm_list)
        .assert_ok();

    // still same balance, subtraction is done manually once per month
    b_mock_rc.borrow().check_esdt_balance(
        subscriber_setup.sub_wrapper.address_ref(),
        WEGLD_TOKEN_ID,
        &rust_biguint!(1_000),
    );
}

#[test]
fn claim_boosted_rewards_for_user_multiple_farms_test() {
    let (
        b_mock_rc,
        mex_pair_setup,
        _stable_pair_setup,
        mut farm_setup,
        mut subscription_setup,
        mut subscriber_setup,
    ) = init_all(
        pair::contract_obj,
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
        subscription_fee::contract_obj,
        farm_boosted_rewards_subscriber::contract_obj,
    );

    let mut farm_setup2 = FarmSetup::new(
        b_mock_rc.clone(),
        &subscriber_setup.owner_addr,
        REWARD_TOKEN_ID,
        LOCKED_TOKEN_ID,
        LP_TOKEN_ID,
        7_500u64,
        mex_pair_setup.pair_wrapper.address_ref(),
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
    );
    let farm_id1 = subscriber_setup.call_add_farm(farm_setup.farm_wrapper.address_ref());
    let farm_id2 = subscriber_setup.call_add_farm(farm_setup2.farm_wrapper.address_ref());
    let farm_list = vec![farm_id1, farm_id2];

    let user = b_mock_rc
        .borrow_mut()
        .create_user_account(&rust_biguint!(0));
    let user_id = 1;
    b_mock_rc.borrow_mut().set_block_epoch(2);

    subscriber_setup
        .call_register_service(vec![
            (
                Some(WEGLD_TOKEN_ID.to_vec()),
                1_000,
                false,
                WEEKLY_SUBSCRIPTION_EPOCHS,
            ),
            (
                Some(WEGLD_TOKEN_ID.to_vec()),
                500,
                false,
                WEEKLY_SUBSCRIPTION_EPOCHS,
            ),
        ])
        .assert_ok();

    subscription_setup
        .call_approve_service(subscriber_setup.sub_wrapper.address_ref())
        .assert_ok();

    b_mock_rc
        .borrow_mut()
        .set_esdt_balance(&user, WEGLD_TOKEN_ID, &rust_biguint!(1_000_000));

    subscription_setup
        .call_deposit(&user, WEGLD_TOKEN_ID, 1_000_000)
        .assert_ok();

    subscription_setup
        .call_subscribe(&user, vec![(1, STANDARD_SERVICE), (1, PREMIUM_SERVICE)])
        .assert_ok();

    // Generate farm rewards
    let user_token_amount = 100_000_000;
    b_mock_rc.borrow_mut().set_esdt_balance(
        &user,
        LP_TOKEN_ID,
        &rust_biguint!(user_token_amount * 2),
    );

    farm_setup.set_user_energy(&user, 1_000, 2, 1);
    farm_setup.enter_farm(&user, LP_TOKEN_ID, user_token_amount);
    let _ = farm_setup.claim_rewards(&user, 1, user_token_amount);

    farm_setup2.set_user_energy(&user, 1_000, 2, 1);
    farm_setup2.enter_farm(&user, LP_TOKEN_ID, user_token_amount);
    let _ = farm_setup2.claim_rewards(&user, 1, user_token_amount);

    b_mock_rc.borrow_mut().set_block_nonce(10);
    b_mock_rc.borrow_mut().set_block_epoch(6);

    farm_setup.set_user_energy(&user, 1_000, 6, 1);
    farm_setup.claim_rewards(&user, 2, user_token_amount);
    farm_setup2.set_user_energy(&user, 1_000, 6, 1);
    farm_setup2.claim_rewards(&user, 2, user_token_amount);

    let farm1_base_rewards = 7_500;
    let farm2_base_rewards = 2_500;
    b_mock_rc
        .borrow()
        .check_nft_balance::<LockedTokenAttributes<DebugApi>>(
            &user,
            LOCKED_TOKEN_ID,
            1,
            &rust_biguint!(farm1_base_rewards + farm2_base_rewards),
            None,
        );

    // Set users energy below threshold
    b_mock_rc.borrow_mut().set_block_epoch(10);
    farm_setup.set_user_energy(&user, 900, 10, 1);
    farm_setup2.set_user_energy(&user, 900, 10, 1);

    farm_setup
        .call_allow_external_claim_boosted_rewards(&user, true)
        .assert_ok();
    farm_setup2
        .call_allow_external_claim_boosted_rewards(&user, true)
        .assert_ok();

    // Call subscriber action
    subscriber_setup
        .call_subtract_payment(vec![user_id])
        .assert_ok();
    subscriber_setup
        .call_perform_claim_boosted(user_id, farm_list.clone())
        .assert_ok();

    // Check that the subscriber claimed the boosted amount for the user, for both farms
    let farm1_boosted_rewards = 2_500;
    let farm2_boosted_rewards = 7_500;
    b_mock_rc
        .borrow()
        .check_nft_balance::<LockedTokenAttributes<DebugApi>>(
            &user,
            LOCKED_TOKEN_ID,
            1,
            &rust_biguint!(
                farm1_base_rewards
                    + farm2_base_rewards
                    + farm1_boosted_rewards
                    + farm2_boosted_rewards
            ), // base rewards are minted with the same nonce
            None,
        );

    // TODO - Check
    b_mock_rc.borrow().check_esdt_balance(
        subscriber_setup.sub_wrapper.address_ref(),
        WEGLD_TOKEN_ID,
        &rust_biguint!(1_000),
    );

    // try perform operation again, same epoch
    subscriber_setup
        .call_perform_claim_boosted(user_id, farm_list.clone())
        .assert_ok();

    // user has the same balance
    b_mock_rc
        .borrow()
        .check_nft_balance::<LockedTokenAttributes<DebugApi>>(
            &user,
            LOCKED_TOKEN_ID,
            1,
            &rust_biguint!(
                farm1_base_rewards
                    + farm2_base_rewards
                    + farm1_boosted_rewards
                    + farm2_boosted_rewards
            ),
            None,
        );

    // still same subscriber balance, no funds subtracted
    b_mock_rc.borrow().check_esdt_balance(
        subscriber_setup.sub_wrapper.address_ref(),
        WEGLD_TOKEN_ID,
        &rust_biguint!(1_000),
    );

    b_mock_rc.borrow_mut().set_block_epoch(11);

    subscriber_setup
        .call_perform_claim_boosted(user_id, farm_list)
        .assert_ok();

    // still same balance, subtraction is done manually once per month
    b_mock_rc.borrow().check_esdt_balance(
        subscriber_setup.sub_wrapper.address_ref(),
        WEGLD_TOKEN_ID,
        &rust_biguint!(1_000),
    );
}

#[test]
fn claim_boosted_rewards_for_premium_user_test() {
    let (
        b_mock_rc,
        _mex_pair_setup,
        _stable_pair_setup,
        mut farm_setup,
        mut subscription_setup,
        mut subscriber_setup,
    ) = init_all(
        pair::contract_obj,
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
        subscription_fee::contract_obj,
        farm_boosted_rewards_subscriber::contract_obj,
    );

    let farm_id = subscriber_setup.call_add_farm(farm_setup.farm_wrapper.address_ref());
    let farm_list = vec![farm_id];

    let user = b_mock_rc
        .borrow_mut()
        .create_user_account(&rust_biguint!(0));
    let user_id = 1;
    b_mock_rc.borrow_mut().set_block_epoch(2);

    subscriber_setup
        .call_register_service(vec![
            (
                Some(WEGLD_TOKEN_ID.to_vec()),
                1_000,
                false,
                WEEKLY_SUBSCRIPTION_EPOCHS,
            ),
            (
                Some(WEGLD_TOKEN_ID.to_vec()),
                500,
                false,
                WEEKLY_SUBSCRIPTION_EPOCHS,
            ),
        ])
        .assert_ok();

    subscription_setup
        .call_approve_service(subscriber_setup.sub_wrapper.address_ref())
        .assert_ok();

    b_mock_rc
        .borrow_mut()
        .set_esdt_balance(&user, WEGLD_TOKEN_ID, &rust_biguint!(1_000_000));

    subscription_setup
        .call_deposit(&user, WEGLD_TOKEN_ID, 1_000_000)
        .assert_ok();

    // Subscribe to premium service
    subscription_setup
        .call_subscribe(&user, vec![(1, STANDARD_SERVICE), (1, PREMIUM_SERVICE)])
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

    subscriber_setup
        .call_subtract_payment(vec![user_id])
        .assert_ok();
    subscriber_setup
        .call_perform_claim_boosted(user_id, farm_list.clone())
        .assert_ok();

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

    // Different price for premium users
    b_mock_rc.borrow().check_esdt_balance(
        subscriber_setup.sub_wrapper.address_ref(),
        WEGLD_TOKEN_ID,
        &rust_biguint!(500),
    );

    // try perform operation again, same epoch
    subscriber_setup
        .call_perform_claim_boosted(user_id, farm_list.clone())
        .assert_ok();

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
        WEGLD_TOKEN_ID,
        &rust_biguint!(500),
    );

    b_mock_rc.borrow_mut().set_block_epoch(11);

    subscriber_setup
        .call_perform_claim_boosted(user_id, farm_list)
        .assert_ok();

    // still same balance, subtraction is done manually once per month
    b_mock_rc.borrow().check_esdt_balance(
        subscriber_setup.sub_wrapper.address_ref(),
        WEGLD_TOKEN_ID,
        &rust_biguint!(500),
    );
}

#[test]
fn mex_operation_with_claim_fees_test() {
    let (
        b_mock_rc,
        _mex_pair_setup,
        _stable_pair_setup,
        _farm_setup,
        mut subscription_setup,
        mut subscriber_setup,
    ) = init_all(
        pair::contract_obj,
        farm_with_locked_rewards::contract_obj,
        energy_factory::contract_obj,
        subscription_fee::contract_obj,
        farm_boosted_rewards_subscriber::contract_obj,
    );

    let first_user = b_mock_rc
        .borrow_mut()
        .create_user_account(&rust_biguint!(0));
    let second_user = b_mock_rc
        .borrow_mut()
        .create_user_account(&rust_biguint!(0));
    let first_user_id = 1;
    let second_user_id = 2;

    b_mock_rc.borrow_mut().set_block_epoch(2);

    subscriber_setup
        .call_register_service(vec![
            (
                Some(WEGLD_TOKEN_ID.to_vec()),
                1_000,
                false,
                WEEKLY_SUBSCRIPTION_EPOCHS,
            ),
            (
                Some(WEGLD_TOKEN_ID.to_vec()),
                500,
                false,
                WEEKLY_SUBSCRIPTION_EPOCHS,
            ),
        ])
        .assert_ok();

    subscription_setup
        .call_approve_service(subscriber_setup.sub_wrapper.address_ref())
        .assert_ok();

    b_mock_rc
        .borrow_mut()
        .set_esdt_balance(&first_user, WEGLD_TOKEN_ID, &rust_biguint!(1_000_000));

    subscription_setup
        .call_deposit(&first_user, WEGLD_TOKEN_ID, 1_000_000)
        .assert_ok();

    b_mock_rc.borrow_mut().set_esdt_balance(
        &second_user,
        WEGLD_TOKEN_ID,
        &rust_biguint!(1_000_000),
    );

    subscription_setup
        .call_deposit(&second_user, WEGLD_TOKEN_ID, 1_000_000)
        .assert_ok();

    // Subscribe to standard service
    subscription_setup
        .call_subscribe(
            &first_user,
            vec![(1, STANDARD_SERVICE), (1, PREMIUM_SERVICE)],
        )
        .assert_ok();
    subscription_setup
        .call_subscribe(
            &second_user,
            vec![(1, STANDARD_SERVICE), (1, PREMIUM_SERVICE)],
        )
        .assert_ok();

    subscriber_setup
        .call_subtract_payment(vec![first_user_id, second_user_id])
        .assert_ok();

    subscriber_setup
        .call_perform_mex_operation(STANDARD_SERVICE, vec![first_user_id, second_user_id])
        .assert_ok();

    // Expected locked tokens balance: 1799
    b_mock_rc
        .borrow()
        .check_nft_balance::<LockedTokenAttributes<DebugApi>>(
            &first_user,
            LOCKED_TOKEN_ID,
            1,
            &rust_biguint!(1799),
            None,
        );

    // Expected locked tokens balance: 1800 - the last user is computed by difference
    b_mock_rc
        .borrow()
        .check_nft_balance::<LockedTokenAttributes<DebugApi>>(
            &second_user,
            LOCKED_TOKEN_ID,
            1,
            &rust_biguint!(1800),
            None,
        );

    b_mock_rc.borrow().check_esdt_balance(
        &subscriber_setup.owner_addr,
        WEGLD_TOKEN_ID,
        &rust_biguint!(0),
    );

    let total_fee_limit_per_week = 100;
    subscriber_setup
        .call_add_max_fee_withdraw_per_week(total_fee_limit_per_week)
        .assert_ok();
    let total_expected_fee_amount = 160;
    b_mock_rc.borrow_mut().set_block_epoch(7);
    subscriber_setup
        .call_claim_fees(total_fee_limit_per_week)
        .assert_ok();
    b_mock_rc.borrow_mut().set_block_epoch(15);
    subscriber_setup
        .call_claim_fees(total_expected_fee_amount - total_fee_limit_per_week)
        .assert_ok();

    b_mock_rc.borrow().check_esdt_balance(
        &subscriber_setup.owner_addr,
        WEGLD_TOKEN_ID,
        &rust_biguint!(total_expected_fee_amount),
    );

    // Check that the fees storage is empty
    let _ = b_mock_rc.borrow_mut().execute_tx(
        &subscriber_setup.owner_addr,
        &subscriber_setup.sub_wrapper,
        &rust_biguint!(0),
        |sc| {
            let total_fees = sc.total_fees().get();
            assert_eq!(total_fees, managed_biguint!(0));
        },
    );
}

#[test]
fn subtract_worth_of_stable_payment_test() {
    let (
        b_mock_rc,
        _mex_pair_setup,
        _stable_pair_setup,
        _farm_setup,
        mut subscription_setup,
        mut subscriber_setup,
    ) = init_all(
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
                Some(WEGLD_TOKEN_ID.to_vec()),
                500,
                true,
                WEEKLY_SUBSCRIPTION_EPOCHS,
            ),
            (
                Some(WEGLD_TOKEN_ID.to_vec()),
                100,
                true,
                WEEKLY_SUBSCRIPTION_EPOCHS,
            ),
        ])
        .assert_ok();

    subscription_setup
        .call_approve_service(subscriber_setup.sub_wrapper.address_ref())
        .assert_ok();

    b_mock_rc
        .borrow_mut()
        .set_esdt_balance(&user, WEGLD_TOKEN_ID, &rust_biguint!(1_000_000));

    subscription_setup
        .call_deposit(&user, WEGLD_TOKEN_ID, 1_000_000)
        .assert_ok();

    subscription_setup
        .call_subscribe(&user, vec![(1, STANDARD_SERVICE), (1, PREMIUM_SERVICE)])
        .assert_ok();

    b_mock_rc.borrow().check_esdt_balance(
        subscriber_setup.sub_wrapper.address_ref(),
        WEGLD_TOKEN_ID,
        &rust_biguint!(0),
    );

    // Pool ratio is 1:2, so for a service payment of 500 USDC worth of WEGLD, the amount should be 250
    subscriber_setup
        .call_subtract_payment(vec![user_id])
        .assert_ok();

    b_mock_rc.borrow().check_esdt_balance(
        subscriber_setup.sub_wrapper.address_ref(),
        WEGLD_TOKEN_ID,
        &rust_biguint!(250),
    );
}
