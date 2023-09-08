use auto_farm::common::address_to_id_mapper::{AddressToIdMapper, NULL_ID};

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, ManagedVecItem)]
pub struct PaymentType<M: ManagedTypeApi> {
    pub opt_specific_token: Option<EgldOrEsdtTokenIdentifier<M>>,
    pub amount_for_normal: BigUint<M>,
    pub amount_for_premium: BigUint<M>,
}

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, ManagedVecItem)]
pub struct ServiceInfo<M: ManagedTypeApi> {
    pub sc_address: ManagedAddress<M>,
    pub payment_type: PaymentType<M>,
    pub endpoint_name: ManagedBuffer<M>,
    pub opt_endpoint_payment: Option<TokenIdentifier<M>>,
    pub opt_interpret_results_endpoint: Option<ManagedBuffer<M>>,
}

#[derive(TypeAbi, TopEncode, TopDecode)]
pub enum SubscriptionType {
    None,
    Daily,
    Weekly,
    Monthly,
}

#[multiversx_sc::module]
pub trait ServiceModule: crate::fees::FeesModule {
    #[endpoint(registerService)]
    fn register_service(&self) {
        let service_address = self.blockchain().get_caller();
        let existing_service_id = self.service_id().get_id(&service_address);
        require!(existing_service_id == NULL_ID, "Service already registered");

        let _ = self.pending_services().insert(service_address);
    }

    #[endpoint(unregisterService)]
    fn unregister_service(&self) {
        let service_address = self.blockchain().get_caller();
        let _ = self.service_id().remove_by_address(&service_address);
        let _ = self.pending_services().swap_remove(&service_address);
    }

    #[only_owner]
    #[endpoint(approveService)]
    fn approve_service(&self, service_address: ManagedAddress) {
        require!(
            self.pending_services().contains(&service_address),
            "Unknown service"
        );

        let _ = self.pending_services().swap_remove(&service_address);
        let _ = self.service_id().insert_new(&service_address);
    }

    #[storage_mapper("serviceId")]
    fn service_id(&self) -> AddressToIdMapper<Self::Api>;

    #[view(getPendingServices)]
    #[storage_mapper("pendingServices")]
    fn pending_services(&self) -> UnorderedSetMapper<ManagedAddress>;
}
