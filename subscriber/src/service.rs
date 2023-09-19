use subscription_fee::service::ProxyTrait as _;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[multiversx_sc::module]
pub trait ServiceModule: crate::common_storage::CommonStorageModule {
    /// Arguments are pairs of sc_address, opt_payment_token and payment_amount
    #[only_owner]
    #[endpoint(registerService)]
    fn register_service(
        &self,
        args: MultiValueEncoded<
            MultiValue3<ManagedAddress, Option<EgldOrEsdtTokenIdentifier>, BigUint>,
        >,
    ) {
        let fees_contract_address = self.fees_contract_address().get();
        let _: () = self
            .register_service_proxy_obj(fees_contract_address)
            .register_service(args)
            .execute_on_dest_context();
    }

    #[only_owner]
    #[endpoint(unregisterService)]
    fn unregister_service(&self) {
        let fees_contract_address = self.fees_contract_address().get();
        let _: () = self
            .register_service_proxy_obj(fees_contract_address)
            .unregister_service()
            .execute_on_dest_context();
    }

    #[proxy]
    fn register_service_proxy_obj(
        &self,
        sc_address: ManagedAddress,
    ) -> subscription_fee::Proxy<Self::Api>;
}
