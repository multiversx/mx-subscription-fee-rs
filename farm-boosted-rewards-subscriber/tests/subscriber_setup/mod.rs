use std::{cell::RefCell, rc::Rc};

use common_subscriber::CommonSubscriberModule;
use energy_query::EnergyQueryModule;
use farm_boosted_rewards_subscriber::{
    claim_farm_boosted::ClaimFarmBoostedRewardsModule, service::ServiceModule,
    subscriber_config::MexActionsPercentages, SubscriberContractMain,
};
use multiversx_sc::{
    codec::multi_types::MultiValue2,
    types::{Address, ManagedVec, MultiValueEncoded}, storage::mappers::AddressId,
};
use multiversx_sc_scenario::{
    managed_address, managed_biguint, managed_token_id, rust_biguint,
    testing_framework::{BlockchainStateWrapper, ContractObjWrapper, TxResult},
    DebugApi,
};

pub const ENERGT_THRESHOLD: u64 = 1_000;
pub const LOCKING_PERIOD: u64 = 1_440;

pub struct SubscriberSetup<SubscriberObjBuilder>
where
    SubscriberObjBuilder:
        'static + Copy + Fn() -> farm_boosted_rewards_subscriber::ContractObj<DebugApi>,
{
    pub b_mock: Rc<RefCell<BlockchainStateWrapper>>,
    pub owner_addr: Address,
    pub sub_wrapper: ContractObjWrapper<
        farm_boosted_rewards_subscriber::ContractObj<DebugApi>,
        SubscriberObjBuilder,
    >,
}

impl<SubscriberObjBuilder> SubscriberSetup<SubscriberObjBuilder>
where
    SubscriberObjBuilder:
        'static + Copy + Fn() -> farm_boosted_rewards_subscriber::ContractObj<DebugApi>,
{
    pub fn new(
        b_mock: Rc<RefCell<BlockchainStateWrapper>>,
        builder: SubscriberObjBuilder,
        fee_contract_address: &Address,
        pair_address: &Address,
        energy_factory_address: &Address,
        owner_addr: &Address,
        reward_token_id: &[u8],
    ) -> Self {
        let rust_zero = rust_biguint!(0);
        let sub_wrapper = b_mock.borrow_mut().create_sc_account(
            &rust_zero,
            Some(owner_addr),
            builder,
            "some other wasm path",
        );

        b_mock
            .borrow_mut()
            .execute_tx(owner_addr, &sub_wrapper, &rust_zero, |sc| {
                let standard_mex_actions_percentages = MexActionsPercentages {
                    lock: 9_000,
                    fees: 800,
                    burn: 200,
                };
                let premium_mex_actions_percentages = MexActionsPercentages {
                    lock: 9_500,
                    fees: 300,
                    burn: 200,
                };

                sc.init(
                    managed_address!(fee_contract_address),
                    managed_biguint!(ENERGT_THRESHOLD),
                    managed_token_id!(reward_token_id),
                    standard_mex_actions_percentages,
                    premium_mex_actions_percentages,
                    managed_address!(energy_factory_address),
                    managed_address!(pair_address),
                    LOCKING_PERIOD,
                );

                sc.set_energy_factory_address(managed_address!(energy_factory_address));
            })
            .assert_ok();

        Self {
            b_mock,
            owner_addr: owner_addr.clone(),
            sub_wrapper,
        }
    }

    pub fn call_register_service(&mut self, args: Vec<(Option<Vec<u8>>, u64, u64)>) -> TxResult {
        self.b_mock.borrow_mut().execute_tx(
            &self.owner_addr,
            &self.sub_wrapper,
            &rust_biguint!(0),
            |sc| {
                let mut args_encoded = MultiValueEncoded::new();
                for arg in args {
                    let (opt_token_id, value, subscription_epochs) = arg;
                    args_encoded.push(
                        (
                            opt_token_id.map(|token_id| managed_token_id!(token_id)),
                            managed_biguint!(value),
                            subscription_epochs,
                        )
                            .into(),
                    );
                }

                sc.register_service(args_encoded);
            },
        )
    }

    // pub fn call_unregister_service(&mut self) -> TxResult {
    //     self.b_mock.borrow_mut().execute_tx(
    //         &self.owner_addr,
    //         &self.sub_wrapper,
    //         &rust_biguint!(0),
    //         |sc| {
    //             sc.unregister_service();
    //         },
    //     )
    // }

    pub fn call_add_farm(&mut self, farm_address: &Address) -> u64 {
        let mut farm_id = 0u64;
        let _ = self.b_mock.borrow_mut().execute_tx(
            &self.owner_addr,
            &self.sub_wrapper,
            &rust_biguint!(0),
            |sc| {
                farm_id = sc.add_farm(managed_address!(farm_address));
            },
        );

        farm_id
    }

    pub fn call_subtract_payment(
        &mut self,
        service_index: usize,
        user_ids_vec: Vec<AddressId>,
    ) -> TxResult {
        self.b_mock.borrow_mut().execute_tx(
            &self.owner_addr,
            &self.sub_wrapper,
            &rust_biguint!(0),
            |sc| {
                let mut user_ids = MultiValueEncoded::new();
                for user_id in user_ids_vec {
                    user_ids.push(user_id);
                }
                sc.subtract_payment_endpoint(service_index, user_ids);
            },
        )
    }

    pub fn call_perform_claim_boosted(
        &mut self,
        service_index: usize,
        user_id: AddressId,
        farms_list: Vec<AddressId>,
    ) -> TxResult {
        self.b_mock.borrow_mut().execute_tx(
            &self.owner_addr,
            &self.sub_wrapper,
            &rust_biguint!(0),
            |sc| {
                let mut user_farms_pairs_to_claim = MultiValueEncoded::new();
                let user_farms = ManagedVec::from(farms_list);
                user_farms_pairs_to_claim.push(MultiValue2::from((user_id, user_farms)));

                sc.perform_claim_rewards_operations_endpoint(
                    service_index,
                    user_farms_pairs_to_claim,
                );
            },
        )
    }
}
