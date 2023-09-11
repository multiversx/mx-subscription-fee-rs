use auto_farm::common::address_to_id_mapper::AddressId;

use crate::service::ServiceInfo;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub trait AllBaseTraits = crate::service::ServiceModule
    + crate::user_tokens::UserTokensModule
    + crate::common_storage::CommonStorageModule
    + energy_query::EnergyQueryModule
    + multiversx_sc_modules::ongoing_operation::OngoingOperationModule;

#[derive(TypeAbi, TopEncode, TopDecode)]
pub struct InterpretedResult<M: ManagedTypeApi> {
    pub opt_new_token: Option<EsdtTokenPayment<M>>,
    pub user_rewards: ManagedVec<M, EsdtTokenPayment<M>>,
}

pub trait SubscriberContract {
    type SubSc: AllBaseTraits;
    type AdditionalDataType: ManagedVecItem + Clone;

    fn perform_action(
        sc: &Self::SubSc,
        user_address: ManagedAddress<<Self::SubSc as ContractBase>::Api>,
        user_id: AddressId,
        service_info: &ServiceInfo<<Self::SubSc as ContractBase>::Api>,
        additional_data: &<Self as SubscriberContract>::AdditionalDataType,
    ) -> Result<InterpretedResult<<Self::SubSc as ContractBase>::Api>, ()>;
}
