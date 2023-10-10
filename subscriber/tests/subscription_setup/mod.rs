#![allow(deprecated)]

use std::{cell::RefCell, rc::Rc};

use auto_farm::common::address_to_id_mapper::AddressId;
use multiversx_sc::types::{Address, MultiValueEncoded};
use multiversx_sc_scenario::{
    managed_address, managed_biguint, managed_token_id, rust_biguint,
    testing_framework::{BlockchainStateWrapper, ContractObjWrapper, TxResult},
    DebugApi,
};
use subscription_fee::{fees::FeesModule, service::ServiceModule, SubscriptionFee};

use crate::{USDC_TOKEN_ID, WEGLD_TOKEN_ID};
pub const MAX_USER_DEPOSITS: usize = 5;
pub const MIN_USER_DEPOSIT_VALUE: u64 = 1_000_000;
pub const MAX_PENDING_SERVICES: usize = 5;
pub const MAX_SERVICE_INFO_NO: usize = 5;

pub struct SubscriptionSetup<SubscriptionObjBuilder>
where
    SubscriptionObjBuilder: 'static + Copy + Fn() -> subscription_fee::ContractObj<DebugApi>,
{
    pub b_mock: Rc<RefCell<BlockchainStateWrapper>>,
    pub owner_addr: Address,
    pub s_wrapper:
        ContractObjWrapper<subscription_fee::ContractObj<DebugApi>, SubscriptionObjBuilder>,
}

impl<SubscriptionObjBuilder> SubscriptionSetup<SubscriptionObjBuilder>
where
    SubscriptionObjBuilder: 'static + Copy + Fn() -> subscription_fee::ContractObj<DebugApi>,
{
    pub fn new(
        b_mock: Rc<RefCell<BlockchainStateWrapper>>,
        builder: SubscriptionObjBuilder,
        owner_addr: &Address,
        pair_address: &Address,
        accepted_tokens: Vec<Vec<u8>>,
    ) -> Self {
        let rust_zero = rust_biguint!(0);
        let s_wrapper = b_mock.borrow_mut().create_sc_account(
            &rust_zero,
            Some(owner_addr),
            builder,
            "some wasm path",
        );

        b_mock
            .borrow_mut()
            .execute_tx(owner_addr, &s_wrapper, &rust_zero, |sc| {
                let mut args = MultiValueEncoded::new();
                for arg in accepted_tokens {
                    let token_id = managed_token_id!(arg);
                    args.push(token_id);
                }

                sc.init(
                    managed_token_id!(USDC_TOKEN_ID),
                    managed_token_id!(WEGLD_TOKEN_ID),
                    MAX_USER_DEPOSITS,
                    managed_biguint!(MIN_USER_DEPOSIT_VALUE),
                    MAX_PENDING_SERVICES,
                    MAX_SERVICE_INFO_NO,
                    managed_address!(pair_address),
                    args,
                )
            })
            .assert_ok();

        Self {
            b_mock,
            owner_addr: owner_addr.clone(),
            s_wrapper,
        }
    }

    pub fn call_approve_service(&mut self, service_address: &Address) -> TxResult {
        self.b_mock.borrow_mut().execute_tx(
            &self.owner_addr,
            &self.s_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.approve_service(managed_address!(service_address));
            },
        )
    }

    pub fn call_deposit(&mut self, caller: &Address, token_id: &[u8], amount: u64) -> TxResult {
        self.b_mock.borrow_mut().execute_esdt_transfer(
            caller,
            &self.s_wrapper,
            token_id,
            0,
            &rust_biguint!(amount),
            |sc| {
                sc.deposit();
            },
        )
    }

    pub fn call_subscribe(&mut self, caller: &Address, args: Vec<(AddressId, usize)>) -> TxResult {
        self.b_mock
            .borrow_mut()
            .execute_tx(caller, &self.s_wrapper, &rust_biguint!(0), |sc| {
                let mut managed_args = MultiValueEncoded::new();
                for arg in args {
                    managed_args.push((arg.0, arg.1).into());
                }

                sc.subscribe(managed_args);
            })
    }
}
