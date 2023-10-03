#![allow(deprecated)]

use std::cell::RefCell;
use std::rc::Rc;

use multiversx_sc::codec::multi_types::OptionalValue;
use multiversx_sc::storage::mappers::StorageTokenWrapper;
use multiversx_sc::types::{Address, BigInt, EsdtLocalRole, ManagedAddress, MultiValueEncoded};
use multiversx_sc_scenario::testing_framework::TxResult;
use multiversx_sc_scenario::{
    managed_address, managed_biguint, managed_token_id, rust_biguint,
    testing_framework::{BlockchainStateWrapper, ContractObjWrapper},
    DebugApi,
};

use config::ConfigModule;
use energy_factory::{energy::EnergyModule, *};
use energy_query::{Energy, EnergyQueryModule};
use farm_boosted_yields::{
    boosted_yields_factors::BoostedYieldsFactorsModule, FarmBoostedYieldsModule,
};
use farm_token::FarmTokenModule;
use farm_with_locked_rewards::*;
use locking_module::lock_with_energy_module::LockWithEnergyModule;
use pausable::{PausableModule, State};
use sc_whitelist_module::SCWhitelistModule;
use simple_lock::locked_token::LockedTokenModule;

const DIV_SAFETY: u64 = 1_000_000_000_000_000_000;
const PER_BLOCK_REWARD_AMOUNT: u64 = 1_000;
const EPOCHS_IN_YEAR: u64 = 360;
static LOCK_OPTIONS: &[u64] = &[EPOCHS_IN_YEAR, 2 * EPOCHS_IN_YEAR, 4 * EPOCHS_IN_YEAR];
static PENALTY_PERCENTAGES: &[u64] = &[4_000, 6_000, 8_000];
const BOOSTED_YIELDS_PERCENTAGE: u64 = 2_500; // 25%
const MAX_REWARDS_FACTOR: u64 = 10;
const USER_REWARDS_ENERGY_CONST: u64 = 3;
const USER_REWARDS_FARM_CONST: u64 = 2;
const MIN_ENERGY_AMOUNT_FOR_BOOSTED_YIELDS: u64 = 1;
const MIN_FARM_AMOUNT_FOR_BOOSTED_YIELDS: u64 = 1;

static FARM_TOKEN_ID: &[u8] = b"FARM-123456";

pub struct FarmSetup<FarmObjBuilder, EnergyFactoryBuilder>
where
    FarmObjBuilder: 'static + Copy + Fn() -> farm_with_locked_rewards::ContractObj<DebugApi>,
    EnergyFactoryBuilder: 'static + Copy + Fn() -> energy_factory::ContractObj<DebugApi>,
{
    pub b_mock: Rc<RefCell<BlockchainStateWrapper>>,
    pub farm_wrapper:
        ContractObjWrapper<farm_with_locked_rewards::ContractObj<DebugApi>, FarmObjBuilder>,
    energy_factory_wrapper:
        ContractObjWrapper<energy_factory::ContractObj<DebugApi>, EnergyFactoryBuilder>,
}

impl<FarmObjBuilder, EnergyFactoryBuilder> FarmSetup<FarmObjBuilder, EnergyFactoryBuilder>
where
    FarmObjBuilder: 'static + Copy + Fn() -> farm_with_locked_rewards::ContractObj<DebugApi>,
    EnergyFactoryBuilder: 'static + Copy + Fn() -> energy_factory::ContractObj<DebugApi>,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        b_mock: Rc<RefCell<BlockchainStateWrapper>>,
        owner: &Address,
        reward_token_id: &[u8],
        locked_token_id: &[u8],
        farming_token_id: &[u8],
        pair_address: &Address,
        farm_builder: FarmObjBuilder,
        energy_factory_builder: EnergyFactoryBuilder,
    ) -> Self {
        let rust_zero = rust_biguint!(0u64);
        let farm_wrapper = b_mock.borrow_mut().create_sc_account(
            &rust_zero,
            Some(owner),
            farm_builder,
            "farm-with-locked-rewards",
        );

        let energy_factory_wrapper = b_mock.borrow_mut().create_sc_account(
            &rust_zero,
            Some(owner),
            energy_factory_builder,
            "energy-factory",
        );

        b_mock
            .borrow_mut()
            .execute_tx(&owner, &energy_factory_wrapper, &rust_zero, |sc| {
                let mut lock_options = MultiValueEncoded::new();
                for (option, penalty) in LOCK_OPTIONS.iter().zip(PENALTY_PERCENTAGES.iter()) {
                    lock_options.push((*option, *penalty).into());
                }

                sc.init(
                    managed_token_id!(reward_token_id),
                    managed_token_id!(reward_token_id),
                    managed_address!(energy_factory_wrapper.address_ref()),
                    0,
                    lock_options,
                );

                sc.locked_token()
                    .set_token_id(managed_token_id!(locked_token_id));
            })
            .assert_ok();

        b_mock
            .borrow_mut()
            .execute_tx(owner, &farm_wrapper, &rust_zero, |sc| {
                sc.init(
                    managed_token_id!(reward_token_id),
                    managed_token_id!(farming_token_id),
                    managed_biguint!(DIV_SAFETY),
                    managed_address!(pair_address),
                    managed_address!(owner),
                    MultiValueEncoded::<DebugApi, ManagedAddress<DebugApi>>::new(),
                );

                sc.farm_token()
                    .set_token_id(managed_token_id!(FARM_TOKEN_ID));
                sc.state().set(State::Active);

                sc.set_locking_sc_address(managed_address!(energy_factory_wrapper.address_ref()));
                sc.set_lock_epochs(EPOCHS_IN_YEAR);

                sc.per_block_reward_amount()
                    .set(&managed_biguint!(PER_BLOCK_REWARD_AMOUNT));

                sc.state().set(State::Active);
                sc.produce_rewards_enabled().set(true);
                sc.set_energy_factory_address(managed_address!(
                    energy_factory_wrapper.address_ref()
                ));
            })
            .assert_ok();

        let lp_token_roles = [EsdtLocalRole::Mint, EsdtLocalRole::Burn];
        b_mock.borrow_mut().set_esdt_local_roles(
            farm_wrapper.address_ref(),
            farming_token_id,
            &lp_token_roles[..],
        );

        let farm_token_roles = [
            EsdtLocalRole::NftCreate,
            EsdtLocalRole::NftAddQuantity,
            EsdtLocalRole::NftBurn,
        ];
        b_mock.borrow_mut().set_esdt_local_roles(
            farm_wrapper.address_ref(),
            FARM_TOKEN_ID,
            &farm_token_roles[..],
        );

        let farming_token_roles = [EsdtLocalRole::Burn];
        b_mock.borrow_mut().set_esdt_local_roles(
            farm_wrapper.address_ref(),
            farming_token_id,
            &farming_token_roles[..],
        );

        let locked_reward_token_roles = [
            EsdtLocalRole::NftCreate,
            EsdtLocalRole::NftAddQuantity,
            EsdtLocalRole::NftBurn,
            EsdtLocalRole::Transfer,
        ];
        b_mock.borrow_mut().set_esdt_local_roles(
            energy_factory_wrapper.address_ref(),
            locked_token_id,
            &locked_reward_token_roles[..],
        );

        b_mock
            .borrow_mut()
            .execute_tx(&owner, &energy_factory_wrapper, &rust_zero, |sc| {
                sc.sc_whitelist_addresses()
                    .add(&managed_address!(farm_wrapper.address_ref()));
            })
            .assert_ok();

        b_mock
            .borrow_mut()
            .execute_tx(&owner, &farm_wrapper, &rust_biguint!(0), |sc| {
                sc.set_boosted_yields_rewards_percentage(BOOSTED_YIELDS_PERCENTAGE);

                sc.set_boosted_yields_factors(
                    managed_biguint!(MAX_REWARDS_FACTOR),
                    managed_biguint!(USER_REWARDS_ENERGY_CONST),
                    managed_biguint!(USER_REWARDS_FARM_CONST),
                    managed_biguint!(MIN_ENERGY_AMOUNT_FOR_BOOSTED_YIELDS),
                    managed_biguint!(MIN_FARM_AMOUNT_FOR_BOOSTED_YIELDS),
                );
            })
            .assert_ok();

        FarmSetup {
            b_mock: b_mock.clone(),
            farm_wrapper,
            energy_factory_wrapper,
        }
    }

    pub fn enter_farm(
        &mut self,
        user: &Address,
        farming_token_id: &[u8],
        farming_token_amount: u64,
    ) {
        self.b_mock
            .borrow_mut()
            .execute_esdt_transfer(
                user,
                &self.farm_wrapper,
                farming_token_id,
                0,
                &rust_biguint!(farming_token_amount),
                |sc| {
                    let enter_farm_result = sc.enter_farm_endpoint(OptionalValue::None);
                    let (out_farm_token, _reward_token) = enter_farm_result.into_tuple();

                    assert_eq!(
                        out_farm_token.amount,
                        managed_biguint!(farming_token_amount)
                    );
                },
            )
            .assert_ok();
    }

    pub fn claim_rewards(
        &mut self,
        user: &Address,
        farm_token_nonce: u64,
        farm_token_amount: u64,
    ) -> u64 {
        let mut result = 0;
        self.b_mock
            .borrow_mut()
            .execute_esdt_transfer(
                user,
                &self.farm_wrapper,
                FARM_TOKEN_ID,
                farm_token_nonce,
                &rust_biguint!(farm_token_amount),
                |sc| {
                    let (out_farm_token, out_reward_token) =
                        sc.claim_rewards_endpoint(OptionalValue::None).into_tuple();
                    assert_eq!(
                        out_farm_token.token_identifier,
                        managed_token_id!(FARM_TOKEN_ID)
                    );
                    assert_eq!(out_farm_token.amount, managed_biguint!(farm_token_amount));

                    result = out_reward_token.amount.to_u64().unwrap();
                },
            )
            .assert_ok();

        result
    }

    pub fn set_user_energy(
        &mut self,
        user: &Address,
        energy: u64,
        last_update_epoch: u64,
        locked_tokens: u64,
    ) {
        self.b_mock
            .borrow_mut()
            .execute_tx(
                user,
                &self.energy_factory_wrapper,
                &rust_biguint!(0),
                |sc| {
                    sc.user_energy(&managed_address!(user)).set(&Energy::new(
                        BigInt::from(managed_biguint!(energy)),
                        last_update_epoch,
                        managed_biguint!(locked_tokens),
                    ));
                },
            )
            .assert_ok();
    }

    pub fn call_allow_external_claim_boosted_rewards(
        &mut self,
        user: &Address,
        allow_external_claim_boosted_rewards: bool,
    ) -> TxResult {
        self.b_mock
            .borrow_mut()
            .execute_tx(user, &self.farm_wrapper, &rust_biguint!(0), |sc| {
                sc.allow_external_claim_boosted_rewards(allow_external_claim_boosted_rewards);
            })
    }
}
