#![no_std]
#![feature(trait_alias)]

use core::marker::PhantomData;

use base_functions::{AllBaseTraits, InterpretedResult, SubscriberContract};
use subscription_fee::service::ServiceInfo;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub mod base_functions;
pub mod base_init;
pub mod common_storage;
pub mod daily_operations;
pub mod service;

pub const DEFAULT_GAS_TO_SAVE_PROGRESS: u64 = 100_000;

#[multiversx_sc::contract]
pub trait SubscriberContractMain:
    base_init::BaseInitModule
    + service::ServiceModule
    + daily_operations::DailyOperationsModule
    + common_storage::CommonStorageModule
    + multiversx_sc_modules::ongoing_operation::OngoingOperationModule
{
    #[init]
    fn init(&self, fees_contract_address: ManagedAddress) {
        self.base_init(fees_contract_address);
    }

    fn dummy_perform_action(&self, service_index: usize) -> OperationCompletionStatus {
        let fees_contract_address = self.fees_contract_address().get();
        let mut user_index = self.get_user_index(&fees_contract_address, 1, service_index);

        let mut dummy_args = ManagedVec::new();
        for _ in 0..10 {
            dummy_args.push(DummyData { dummy_data: 0 });
        }

        let run_result = self.perform_service::<DummyWrapper<Self>>(
            DEFAULT_GAS_TO_SAVE_PROGRESS,
            service_index,
            &mut user_index,
            dummy_args,
        );

        self.user_index().set(user_index);

        run_result
    }
}

pub struct DummyWrapper<T: AllBaseTraits> {
    _phantom: PhantomData<T>,
}

#[derive(Clone, ManagedVecItem)]
pub struct DummyData {
    pub dummy_data: u8,
}

impl<T> SubscriberContract for DummyWrapper<T>
where
    T: AllBaseTraits,
{
    type SubSc = T;
    type AdditionalDataType = DummyData;

    fn perform_action(
        _sc: &Self::SubSc,
        _user_address: ManagedAddress<<Self::SubSc as ContractBase>::Api>,
        _service_index: usize,
        _service_info: &ServiceInfo<<Self::SubSc as ContractBase>::Api>,
        _additional_data: &<Self as SubscriberContract>::AdditionalDataType,
    ) -> Result<InterpretedResult<<Self::SubSc as ContractBase>::Api>, ()> {
        let result = InterpretedResult {
            user_rewards: ManagedVec::new(),
        };

        Result::Ok(result)
    }
}
