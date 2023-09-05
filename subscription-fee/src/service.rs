use auto_farm::common::address_to_id_mapper::{AddressId, AddressToIdMapper, NULL_ID};

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
    pub payment_type: PaymentType<M>,
    pub endpoint_name: ManagedBuffer<M>,
    pub opt_endpoint_payment: Option<EsdtTokenPayment<M>>,
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
    #[only_owner]
    #[endpoint(registerService)]
    fn register_service(
        &self,
        service_address: ManagedAddress,
        payment_type: PaymentType<Self::Api>,
        endpoint_name: ManagedBuffer,
        opt_endpoint_payment: OptionalValue<EsdtTokenPayment>,
        opt_interpret_results_endpoint: OptionalValue<ManagedBuffer>,
    ) {
        let mut service_id = self.service_id().get_id(&service_address);
        if service_id == NULL_ID {
            service_id = self.service_id().insert_new(&service_address);
        }

        if let Option::Some(token_id) = &payment_type.opt_specific_token {
            require!(token_id.is_valid(), "Invalid token");
        }
        require!(
            payment_type.amount_for_normal <= payment_type.amount_for_premium,
            "Invalid amounts"
        );

        let service_info = ServiceInfo {
            payment_type,
            endpoint_name,
            opt_endpoint_payment: opt_endpoint_payment.into_option(),
            opt_interpret_results_endpoint: opt_interpret_results_endpoint.into_option(),
        };
        self.service_info(service_id)
            .update(|services_vec| services_vec.push(service_info));
    }

    #[only_owner]
    #[endpoint(unregisterService)]
    fn unregister_service(&self, service_address: ManagedAddress) {
        let service_id = self.service_id().remove_by_address(&service_address);
        self.service_info(service_id).clear();
        self.subscribed_users(service_id).clear();
    }

    #[view(getServiceInfo)]
    fn get_service_info(
        &self,
        service_address: ManagedAddress,
    ) -> MultiValueEncoded<ServiceInfo<Self::Api>> {
        let service_id = self.service_id().get_id_non_zero(&service_address);

        self.service_info(service_id).get().into()
    }

    // Might be removed if it consumes too much gas
    #[view(getSubscribedUsers)]
    fn get_subscribed_users(
        &self,
        service_address: ManagedAddress,
    ) -> MultiValueEncoded<ManagedAddress> {
        let service_id = self.service_id().get_id_non_zero(&service_address);

        let mapper = self.subscribed_users(service_id);
        let nr_users = mapper.len();
        let mut users = MultiValueEncoded::new();
        for i in 1..=nr_users {
            let user_id = mapper.get_by_index(i);
            let opt_user_address = self.user_ids().get_address(user_id);
            let user_address = unsafe { opt_user_address.unwrap_unchecked() };
            users.push(user_address);
        }

        users
    }

    #[storage_mapper("serviceId")]
    fn service_id(&self) -> AddressToIdMapper<Self::Api>;

    // one service may have multiple options
    #[storage_mapper("serviceInfo")]
    fn service_info(
        &self,
        service_id: AddressId,
    ) -> SingleValueMapper<ManagedVec<ServiceInfo<Self::Api>>>;

    #[storage_mapper("subscribedUsers")]
    fn subscribed_users(&self, service_id: AddressId) -> UnorderedSetMapper<AddressId>;

    #[storage_mapper("subscriptionType")]
    fn subscription_type(
        &self,
        user_id: AddressId,
        service_id: AddressId,
        service_index: usize,
    ) -> SingleValueMapper<SubscriptionType>;
}
