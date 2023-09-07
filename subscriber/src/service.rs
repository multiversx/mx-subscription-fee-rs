use auto_farm::common::address_to_id_mapper::AddressId;

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
pub trait ServiceModule {
    // one service may have multiple options
    #[storage_mapper("serviceInfo")]
    fn service_info(&self) -> SingleValueMapper<ManagedVec<ServiceInfo<Self::Api>>>;

    #[storage_mapper("subscribedUsers")]
    fn subscribed_users(&self, service_index: usize) -> UnorderedSetMapper<AddressId>;

    #[storage_mapper("subscriptionType")]
    fn subscription_type(
        &self,
        user_id: AddressId,
        service_index: usize,
    ) -> SingleValueMapper<SubscriptionType>;
}
