#![no_std]

use core::marker::PhantomData;

use auto_farm::common::address_to_id_mapper::AddressId;
use claim_metaboding::AdditionalMetabodingData;
use metabonding::claim::ClaimArgPair;
use subscriber::{
    base_functions::{AllBaseTraits, InterpretedResult, SubscriberContract},
    daily_operations::Epoch,
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

    /// Only one claim arg per user
    #[only_owner]
    #[endpoint(performAction)]
    fn perform_action_endpoint(
        &self,
        raw_claim_args: MultiValueEncoded<ClaimArgPair<Self::Api>>,
    ) -> OperationCompletionStatus {
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

        let last_action_epoch = self.last_global_action_epoch().get();
        let current_epoch = self.blockchain().get_block_epoch();
        let mut user_index = if last_action_epoch == current_epoch {
            self.user_index().get()
        } else {
            0
        };

        let result = self.perform_service::<Wrapper<Self>>(0, &mut user_index, args_vec);
        self.user_index().set(user_index);

        result
    }

    #[storage_mapper("userIndex")]
    fn user_index(&self) -> SingleValueMapper<usize>;

    #[storage_mapper("lastGloblalActionEpoch")]
    fn last_global_action_epoch(&self) -> SingleValueMapper<Epoch>;
}

pub struct Wrapper<T: AllBaseTraits + claim_metaboding::ClaimMetabondingModule> {
    _phantom: PhantomData<T>,
}

impl<T> SubscriberContract for Wrapper<T>
where
    T: AllBaseTraits + claim_metaboding::ClaimMetabondingModule,
{
    type SubSc = T;
    type AdditionalDataType = AdditionalMetabodingData<<Self::SubSc as ContractBase>::Api>;

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
