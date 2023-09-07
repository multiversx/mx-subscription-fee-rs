use auto_farm::common::address_to_id_mapper::AddressToIdMapper;

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait CommonStorageModule {
    // to be used as "get from address" from the fees_contract
    #[storage_mapper("serviceId")]
    fn service_id(&self) -> AddressToIdMapper<Self::Api>;

    // to be used as "get from address" from the fees_contract
    #[storage_mapper("userId")]
    fn user_id(&self) -> AddressToIdMapper<Self::Api>;

    #[storage_mapper("feesContractAddress")]
    fn fees_contract_address(&self) -> SingleValueMapper<ManagedAddress>;
}
