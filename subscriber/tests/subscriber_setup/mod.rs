use std::{cell::RefCell, rc::Rc};

use multiversx_sc::types::{Address, MultiValueEncoded};
use multiversx_sc_scenario::{
    managed_address, managed_biguint, managed_token_id_wrapped, rust_biguint,
    testing_framework::{BlockchainStateWrapper, ContractObjWrapper, TxResult},
    DebugApi,
};
use subscriber::{service::ServiceModule, SubscriberContractMain};

pub struct SubscriberSetup<SubscriberObjBuilder>
where
    SubscriberObjBuilder: 'static + Copy + Fn() -> subscriber::ContractObj<DebugApi>,
{
    pub b_mock: Rc<RefCell<BlockchainStateWrapper>>,
    pub owner_addr: Address,
    pub sub_wrapper: ContractObjWrapper<subscriber::ContractObj<DebugApi>, SubscriberObjBuilder>,
}

impl<SubscriberObjBuilder> SubscriberSetup<SubscriberObjBuilder>
where
    SubscriberObjBuilder: 'static + Copy + Fn() -> subscriber::ContractObj<DebugApi>,
{
    pub fn new(
        b_mock: Rc<RefCell<BlockchainStateWrapper>>,
        builder: SubscriberObjBuilder,
        fee_contract_address: &Address,
        owner_addr: &Address,
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
                sc.init(managed_address!(fee_contract_address));
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

    pub fn call_unregister_service(&mut self) -> TxResult {
        self.b_mock.borrow_mut().execute_tx(
            &self.owner_addr,
            &self.sub_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.unregister_service();
            },
        )
    }

    pub fn call_perform_action(&mut self, service_index: usize) -> TxResult {
        self.b_mock.borrow_mut().execute_tx(
            &self.owner_addr,
            &self.sub_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.dummy_perform_action(service_index);
            },
        )
    }
}
