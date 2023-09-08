use auto_farm::common::address_to_id_mapper::{AddressId, NULL_ID};

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
}

#[derive(TypeAbi, TopEncode, TopDecode)]
pub enum SubscriptionType {
    None,
    Daily,
    Weekly,
    Monthly,
}

mod register_service_proxy {
    #[multiversx_sc::proxy]
    pub trait RegisterServiceProxy {
        #[endpoint(registerService)]
        fn register_service(&self);
    }
}

#[multiversx_sc::module]
pub trait ServiceModule: crate::common_storage::CommonStorageModule {
    #[only_owner]
    #[endpoint(registerService)]
    fn register_service(&self, sc_address: ManagedAddress, payment_type: PaymentType<Self::Api>) {
        let fees_contract_address = self.fees_contract_address().get();
        let service_address = self.blockchain().get_sc_address();
        let service_id = self
            .service_id()
            .get_id_at_address(&fees_contract_address, &service_address);
        if service_id == NULL_ID {
            let _: () = self
                .register_service_proxy_obj(fees_contract_address)
                .register_service()
                .execute_on_dest_context();
        }

        require!(
            self.blockchain().is_smart_contract(&sc_address),
            "Invalid address"
        );

        if let Option::Some(token_id) = &payment_type.opt_specific_token {
            require!(token_id.is_valid(), "Invalid token");
        }
        require!(
            payment_type.amount_for_normal <= payment_type.amount_for_premium,
            "Invalid amounts"
        );

        let service_info = ServiceInfo {
            sc_address,
            payment_type,
        };
        self.service_info()
            .update(|services_vec| services_vec.push(service_info));
    }

    #[proxy]
    fn register_service_proxy_obj(
        &self,
        sc_address: ManagedAddress,
    ) -> register_service_proxy::Proxy<Self::Api>;

    // one service may have multiple options
    #[view(getServiceInfo)]
    #[storage_mapper("serviceInfo")]
    fn service_info(&self) -> SingleValueMapper<ManagedVec<ServiceInfo<Self::Api>>>;

    #[view(getSubscribedUsers)]
    #[storage_mapper("subscribedUsers")]
    fn subscribed_users(&self, service_index: usize) -> UnorderedSetMapper<AddressId>;

    #[storage_mapper("subscriptionType")]
    fn subscription_type(
        &self,
        user_id: AddressId,
        service_index: usize,
    ) -> SingleValueMapper<SubscriptionType>;
}
