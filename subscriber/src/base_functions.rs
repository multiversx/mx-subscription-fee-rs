use subscription_fee::service::ServiceInfo;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub trait AllBaseTraits = crate::service::ServiceModule
    + crate::common_storage::CommonStorageModule
    + multiversx_sc_modules::ongoing_operation::OngoingOperationModule;

#[derive(TypeAbi, TopEncode, TopDecode)]
pub struct InterpretedResult<M: ManagedTypeApi> {
    pub user_rewards: ManagedVec<M, EsdtTokenPayment<M>>,
}

pub trait SubscriberContract {
    type SubSc: AllBaseTraits;
    type AdditionalDataType: ManagedVecItem + Clone;

    #[allow(clippy::result_unit_err)]
    fn perform_action(
        sc: &Self::SubSc,
        user_address: ManagedAddress<<Self::SubSc as ContractBase>::Api>,
        service_index: usize,
        service_info: &ServiceInfo<<Self::SubSc as ContractBase>::Api>,
        additional_data: &<Self as SubscriberContract>::AdditionalDataType,
    ) -> Result<InterpretedResult<<Self::SubSc as ContractBase>::Api>, ()>;
}
