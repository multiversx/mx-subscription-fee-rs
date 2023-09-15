#![allow(deprecated)]

use multiversx_sc::types::{
    Address, EgldOrEsdtTokenIdentifier, MultiValueEncoded, TokenIdentifier,
};
use multiversx_sc_scenario::{
    managed_address, rust_biguint,
    testing_framework::{BlockchainStateWrapper, ContractObjWrapper},
    DebugApi,
};
use subscription_fee::SubscriptionFee;

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
    pub fn new(
        builder: SubscriptionObjBuilder,
        pair_address: &Address,
        accepted_tokens: Vec<Vec<u8>>,
    ) -> Self {
        let rust_zero = rust_biguint!(0);
        let mut b_mock = BlockchainStateWrapper::new();
        let owner_addr = b_mock.create_user_account(&rust_zero);
        let s_wrapper =
            b_mock.create_sc_account(&rust_zero, Some(&owner_addr), builder, "some wasm path");

        b_mock
            .execute_tx(&owner_addr, &s_wrapper, &rust_zero, |sc| {
                let mut args = MultiValueEncoded::new();
                for arg in accepted_tokens {
                    if &arg == b"EGLD" {
                        let token_id = EgldOrEsdtTokenIdentifier::egld();
                        args.push(token_id);
                    } else {
                        let token_id = TokenIdentifier::from_esdt_bytes(arg);
                        args.push(EgldOrEsdtTokenIdentifier::esdt(token_id));
                    }
                }

                sc.init(managed_address!(pair_address), args);
            })
            .assert_ok();

        Self {
            b_mock,
            owner_addr,
            s_wrapper,
        }
    }
}
