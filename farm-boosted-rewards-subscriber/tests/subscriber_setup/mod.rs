use std::{cell::RefCell, rc::Rc};

use multiversx_sc::types::{Address, MultiValueEncoded};
use multiversx_sc_scenario::{
    managed_address, managed_biguint, managed_token_id, managed_token_id_wrapped, rust_biguint,
    testing_framework::{BlockchainStateWrapper, ContractObjWrapper, TxResult},
    DebugApi,
};

use auto_farm::common::address_to_id_mapper::AddressId;
use farm_boosted_rewards_subscriber::{buy_mex::MexActionsPercentages, SubscriberContractMain};
use subscriber::service::ServiceModule;

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
                    mex_burn: 200,
                };
                let premium_mex_actions_percentages = MexActionsPercentages {
                    lock: 9_500,
                    fees: 300,
                    mex_burn: 200,
                };

                sc.init(
                    managed_address!(fee_contract_address),
                    managed_biguint!(ENERGT_THRESHOLD),
                    managed_token_id!(reward_token_id),
                    standard_mex_actions_percentages,
                    premium_mex_actions_percentages,
                    managed_address!(fee_contract_address), // TODO - simple lock address
                    LOCKING_PERIOD,
                );
            })
            .assert_ok();

        Self {
            b_mock,
            owner_addr: owner_addr.clone(),
            sub_wrapper,
        }
    }

    pub fn call_register_service(
        &mut self,
        args: Vec<(Address, Option<Vec<u8>>, u64)>,
    ) -> TxResult {
        self.b_mock.borrow_mut().execute_tx(
            &self.owner_addr,
            &self.sub_wrapper,
            &rust_biguint!(0),
            |sc| {
                let mut args_encoded = MultiValueEncoded::new();
                for arg in args {
                    let (sc_address, opt_token_id, value) = arg;
                    args_encoded.push(
                        (
                            managed_address!(&sc_address),
                            opt_token_id.map(|token_id| managed_token_id_wrapped!(token_id)),
                            managed_biguint!(value),
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

    pub fn call_subtract_payment(&mut self, service_index: usize) -> TxResult {
        self.b_mock.borrow_mut().execute_tx(
            &self.owner_addr,
            &self.sub_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.subtract_payment_endpoint(service_index);
            },
        )
    }

    pub fn call_perform_action(&mut self, service_index: usize, user_id: AddressId) -> TxResult {
        self.b_mock.borrow_mut().execute_tx(
            &self.owner_addr,
            &self.sub_wrapper,
            &rust_biguint!(0),
            |sc| {
                let mut users = MultiValueEncoded::new();
                users.push(user_id);
                sc.perform_action_endpoint(service_index, users);
            },
        )
    }
}
