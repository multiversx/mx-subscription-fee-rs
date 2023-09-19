#![allow(deprecated)]

use std::cell::RefCell;
use std::rc::Rc;

use multiversx_sc::types::{Address, EsdtLocalRole, ManagedAddress, MultiValueEncoded};
use multiversx_sc_scenario::{
    managed_address, managed_biguint, managed_token_id, rust_biguint,
    testing_framework::{BlockchainStateWrapper, ContractObjWrapper, TxTokenTransfer},
    DebugApi,
};

use pair::*;
use pair::{config::ConfigModule, safe_price::SafePriceModule};
use pausable::{PausableModule, State};

pub struct PairSetup<PairObjBuilder>
where
    PairObjBuilder: 'static + Copy + Fn() -> pair::ContractObj<DebugApi>,
{
    pub b_mock: Rc<RefCell<BlockchainStateWrapper>>,
    pub first_token_id: Vec<u8>,
    pub second_token_id: Vec<u8>,
    pub lp_token_id: Vec<u8>,
    pub pair_wrapper: ContractObjWrapper<pair::ContractObj<DebugApi>, PairObjBuilder>,
}

impl<PairObjBuilder> PairSetup<PairObjBuilder>
where
    PairObjBuilder: 'static + Copy + Fn() -> pair::ContractObj<DebugApi>,
{
    pub fn new(
        b_mock: Rc<RefCell<BlockchainStateWrapper>>,
        pair_builder: PairObjBuilder,
        owner: &Address,
        first_token_id: &[u8],
        second_token_id: &[u8],
        lp_token_id: &[u8],
        first_token_amount: u64,
        second_token_amount: u64,
    ) -> Self {
        let rust_zero = rust_biguint!(0u64);
        let pair_wrapper =
            b_mock
                .borrow_mut()
                .create_sc_account(&rust_zero, Some(owner), pair_builder, "pair");

        b_mock
            .borrow_mut()
            .execute_tx(owner, &pair_wrapper, &rust_zero, |sc| {
                sc.init(
                    managed_token_id!(first_token_id),
                    managed_token_id!(second_token_id),
                    managed_address!(owner),
                    managed_address!(owner),
                    0,
                    0,
                    ManagedAddress::<DebugApi>::zero(),
                    MultiValueEncoded::<DebugApi, ManagedAddress<DebugApi>>::new(),
                );

                sc.lp_token_identifier()
                    .set(&managed_token_id!(lp_token_id));
                sc.state().set(State::Active);
            })
            .assert_ok();

        let lp_token_roles = [EsdtLocalRole::Mint, EsdtLocalRole::Burn];
        b_mock.borrow_mut().set_esdt_local_roles(
            pair_wrapper.address_ref(),
            lp_token_id,
            &lp_token_roles[..],
        );

        let mut pair_setup = PairSetup {
            b_mock: b_mock.clone(),
            first_token_id: first_token_id.to_vec(),
            second_token_id: second_token_id.to_vec(),
            lp_token_id: lp_token_id.to_vec(),
            pair_wrapper,
        };

        b_mock.borrow_mut().set_esdt_balance(
            owner,
            first_token_id,
            &rust_biguint!(first_token_amount),
        );
        b_mock.borrow_mut().set_esdt_balance(
            owner,
            second_token_id,
            &rust_biguint!(second_token_amount),
        );
        pair_setup.add_liquidity(owner, first_token_amount, second_token_amount);

        let mut block_round = 1;
        b_mock.borrow_mut().set_block_round(block_round);

        // setup price observations
        for _i in 1usize..=20 {
            block_round += 1;
            b_mock.borrow_mut().set_block_round(block_round);

            b_mock
                .borrow_mut()
                .execute_tx(owner, &pair_setup.pair_wrapper, &rust_biguint!(0), |sc| {
                    sc.update_safe_price(
                        &managed_biguint!(first_token_amount),
                        &managed_biguint!(second_token_amount),
                    );
                })
                .assert_ok();
        }

        pair_setup
    }

    pub fn add_liquidity(
        &mut self,
        caller: &Address,
        first_token_amount: u64,
        second_token_amount: u64,
    ) {
        let payments = vec![
            TxTokenTransfer {
                token_identifier: self.first_token_id.clone(),
                nonce: 0,
                value: rust_biguint!(first_token_amount),
            },
            TxTokenTransfer {
                token_identifier: self.second_token_id.clone(),
                nonce: 0,
                value: rust_biguint!(second_token_amount),
            },
        ];

        self.b_mock
            .borrow_mut()
            .execute_esdt_multi_transfer(caller, &self.pair_wrapper, &payments, |sc| {
                _ = sc.add_liquidity(managed_biguint!(1), managed_biguint!(1));
            })
            .assert_ok();
    }
}
