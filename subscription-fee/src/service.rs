use auto_farm::common::address_to_id_mapper::{AddressId, AddressToIdMapper, NULL_ID};

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode)]
pub enum PaymentType<M: ManagedTypeApi> {
    SpecificToken {
        token_id: EgldOrEsdtTokenIdentifier<M>,
        amount: BigUint<M>,
    },
    AnyToken {
        amount_in_dollars: BigUint<M>,
    },
}

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, ManagedVecItem)]
pub struct ServiceInfo<M: ManagedTypeApi> {
    pub payment_type: PaymentType<M>,
    pub endpoint_name: ManagedBuffer<M>,
    pub opt_endpoint_payment: Option<EsdtTokenPayment<M>>,
    pub opt_interpret_results_endpoint: Option<ManagedBuffer<M>>,
}

#[derive(TypeAbi, TopEncode, TopDecode)]
pub struct InterpretedResult<M: ManagedTypeApi> {
    pub new_token: EsdtTokenPayment<M>,
    pub user_rewards: ManagedVec<M, EsdtTokenPayment<M>>,
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

        if let PaymentType::SpecificToken {
            token_id,
            amount: _,
        } = &payment_type
        {
            require!(token_id.is_valid(), "Invalid token");
        }

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

    #[storage_mapper("serviceId")]
    fn service_id(&self) -> AddressToIdMapper<Self::Api>;

    #[storage_mapper("serviceInfo")]
    fn service_info(
        &self,
        service_id: AddressId,
    ) -> SingleValueMapper<ManagedVec<ServiceInfo<Self::Api>>>;

    #[storage_mapper("subscribedUsers")]
    fn subscribed_users(&self, service_id: AddressId) -> UnorderedSetMapper<AddressId>;
}
