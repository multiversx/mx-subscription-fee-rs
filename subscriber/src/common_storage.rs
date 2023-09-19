use auto_farm::common::address_to_id_mapper::{AddressId, AddressToIdMapper};
use subscription_fee::service::ServiceInfo;

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait CommonStorageModule {
    #[storage_mapper("feesContractAddress")]
    fn fees_contract_address(&self) -> SingleValueMapper<ManagedAddress>;

    // following storages are to be used as "get from address" from the fees_contract

    #[storage_mapper("serviceId")]
    fn service_id(&self) -> AddressToIdMapper<Self::Api>;

    #[storage_mapper("userId")]
    fn user_id(&self) -> AddressToIdMapper<Self::Api>;

    #[storage_mapper("subscribedUsers")]
    fn subscribed_users(
        &self,
        service_id: AddressId,
        service_index: usize,
    ) -> UnorderedSetMapper<AddressId>;

    #[storage_mapper("serviceInfo")]
    fn service_info(
        &self,
        service_id: AddressId,
    ) -> SingleValueMapper<ManagedVec<ServiceInfo<Self::Api>>>;
}
