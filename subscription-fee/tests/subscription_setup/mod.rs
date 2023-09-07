#![allow(deprecated)]

use multiversx_sc::types::Address;
use multiversx_sc_scenario::{
    rust_biguint,
    testing_framework::{BlockchainStateWrapper, ContractObjWrapper},
    DebugApi,
};

pub struct SubscriptionSetup<SubscriptionObjBuilder>
where
    SubscriptionObjBuilder: 'static + Copy + Fn() -> subscription_fee::ContractObj<DebugApi>,
{
    pub b_mock: BlockchainStateWrapper,
    pub owner_addr: Address,
    pub s_wrapper:
        ContractObjWrapper<subscription_fee::ContractObj<DebugApi>, SubscriptionObjBuilder>,
}

impl<SubscriptionObjBuilder> SubscriptionSetup<SubscriptionObjBuilder>
where
    SubscriptionObjBuilder: 'static + Copy + Fn() -> subscription_fee::ContractObj<DebugApi>,
{
    pub fn new(builder: SubscriptionObjBuilder) -> Self {
        let rust_zero = rust_biguint!(0);
        let mut b_mock = BlockchainStateWrapper::new();
        let owner_addr = b_mock.create_user_account(&rust_zero);
        let s_wrapper =
            b_mock.create_sc_account(&rust_zero, Some(&owner_addr), builder, "some wasm path");

        Self {
            b_mock,
            owner_addr,
            s_wrapper,
        }
    }
}
