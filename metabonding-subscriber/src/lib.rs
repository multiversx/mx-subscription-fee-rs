#![no_std]

use core::marker::PhantomData;

use auto_farm::common::address_to_id_mapper::AddressId;
use claim_metaboding::AdditionalMetabodingData;
use metabonding::claim::ClaimArgPair;
use subscriber::{
    base_functions::{AllBaseTraits, InterpretedResult, SubscriberContract},
    daily_operations::Epoch,
};
use subscription_fee::service::ServiceInfo;

multiversx_sc::imports!();

pub mod claim_metaboding;

#[multiversx_sc::contract]
pub trait MetabondingSubscriber:
    claim_metaboding::ClaimMetabondingModule
    + subscriber::base_init::BaseInitModule
    + subscriber::service::ServiceModule
    + subscriber::daily_operations::DailyOperationsModule
    + subscriber::common_storage::CommonStorageModule
    + energy_query::EnergyQueryModule
    + multiversx_sc_modules::ongoing_operation::OngoingOperationModule
{
    #[init]
    fn init(&self, fees_contract_address: ManagedAddress, energy_threshold: BigUint) {
        self.base_init(fees_contract_address);
        self.energy_threshold().set(&energy_threshold);
    }

    /// Only one claim arg per user
    #[only_owner]
    #[endpoint(performAction)]
    fn perform_action_endpoint(
        &self,
        service_index: usize,
        raw_claim_args: MultiValueEncoded<ClaimArgPair<Self::Api>>,
    ) -> OperationCompletionStatus {
        require!(service_index <= 1, "invalid index");

        let mut args_vec = ManagedVec::new();
        for arg in raw_claim_args {
            let (week, user_delegation_amount, user_lkmex_staked_amount, signature) =
                arg.into_tuple();
            args_vec.push(AdditionalMetabodingData {
                week,
                user_delegation_amount,
                user_lkmex_staked_amount,
                signature,
            });
        }

        let current_epoch = self.blockchain().get_block_epoch();
        let mut user_index = self.get_user_index(service_index, current_epoch);
        let result = self.perform_service::<MetabondingWrapper<Self>>(
            service_index,
            &mut user_index,
            args_vec,
        );

        self.user_index().set(user_index);
        self.last_global_action_epoch(service_index)
            .set(current_epoch);

        result
    }

    fn get_user_index(&self, service_index: usize, current_epoch: Epoch) -> usize {
        let last_action_epoch = self.last_global_action_epoch(service_index).get();
        if last_action_epoch == current_epoch {
            self.user_index().get()
        } else {
            0
        }
    }

    #[storage_mapper("userIndex")]
    fn user_index(&self) -> SingleValueMapper<usize>;

    #[storage_mapper("lastGloblalActionEpoch")]
    fn last_global_action_epoch(&self, service_index: usize) -> SingleValueMapper<Epoch>;
}

pub struct MetabondingWrapper<
    T: AllBaseTraits + claim_metaboding::ClaimMetabondingModule + energy_query::EnergyQueryModule,
> {
    _phantom: PhantomData<T>,
}

impl<T> SubscriberContract for MetabondingWrapper<T>
where
    T: AllBaseTraits + claim_metaboding::ClaimMetabondingModule + energy_query::EnergyQueryModule,
{
    type SubSc = T;
    type AdditionalDataType = AdditionalMetabodingData<<Self::SubSc as ContractBase>::Api>;

    fn perform_action(
        sc: &Self::SubSc,
        user_address: ManagedAddress<<Self::SubSc as ContractBase>::Api>,
        _user_id: AddressId,
        service_index: usize,
        service_info: &ServiceInfo<<Self::SubSc as ContractBase>::Api>,
        additional_data: &<Self as SubscriberContract>::AdditionalDataType,
    ) -> Result<InterpretedResult<<Self::SubSc as ContractBase>::Api>, ()> {
        if service_index == 1 {
            let energy_threshold = sc.energy_threshold().get();
            let user_energy = sc.get_energy_amount(&user_address);
            if user_energy < energy_threshold {
                return Result::Err(());
            }
        }

        let rewards_vec = sc.claim_metaboding_rewards(
            service_info.sc_address.clone(),
            user_address,
            additional_data,
        );
        let result = InterpretedResult {
            user_rewards: rewards_vec,
        };

        Result::Ok(result)
    }
}
