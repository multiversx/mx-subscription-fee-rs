#![no_std]

use core::marker::PhantomData;

use auto_farm::common::address_to_id_mapper::AddressId;
use claim_metaboding::AdditionalMetabodingData;
use subscriber::{
    base_functions::{AllBaseTraits, InterpretedResult, SubscriberContract},
    service::ServiceInfo,
};

multiversx_sc::imports!();

pub mod claim_metaboding;

#[multiversx_sc::contract]
pub trait MetabondingSubscriber:
    claim_metaboding::ClaimMetabondingModule
    + subscriber::base_init::BaseInitModule
    + subscriber::service::ServiceModule
    + subscriber::daily_operations::DailyOperationsModule
    + subscriber::user_tokens::UserTokensModule
    + subscriber::subscription::SubscriptionModule
    + subscriber::common_storage::CommonStorageModule
    + energy_query::EnergyQueryModule
    + multiversx_sc_modules::ongoing_operation::OngoingOperationModule
{
    #[init]
    fn init(
        &self,
        fees_contract_address: ManagedAddress,
        accepted_tokens: MultiValueEncoded<TokenIdentifier>,
    ) {
        self.base_init(fees_contract_address, accepted_tokens);
    }
}

pub struct Wrapper<T: AllBaseTraits + claim_metaboding::ClaimMetabondingModule> {
    _phantom: PhantomData<T>,
}

impl<T> SubscriberContract for Wrapper<T>
where
    T: AllBaseTraits + claim_metaboding::ClaimMetabondingModule,
{
    type SubSc = T;
    type AdditionalDataType = ManagedVec<
        <Self::SubSc as ContractBase>::Api,
        AdditionalMetabodingData<<Self::SubSc as ContractBase>::Api>,
    >;

    fn perform_action(
        sc: &Self::SubSc,
        user_address: ManagedAddress<<Self::SubSc as ContractBase>::Api>,
        _user_id: AddressId,
        service_info: &ServiceInfo<<Self::SubSc as ContractBase>::Api>,
        additional_data: &<Self as SubscriberContract>::AdditionalDataType,
    ) -> Result<InterpretedResult<<Self::SubSc as ContractBase>::Api>, ()> {
        let rewards_vec = sc.claim_metaboding_rewards(
            service_info.sc_address.clone(),
            user_address,
            additional_data,
        );
        let result = InterpretedResult {
            opt_new_token: None,
            user_rewards: rewards_vec,
        };

        Result::Ok(result)
    }
}
