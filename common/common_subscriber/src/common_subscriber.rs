#![no_std]

use subscription_fee::{
    service::ProxyTrait as _,
    subtract_payments::{Epoch, ProxyTrait as _, ScResult},
};

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[derive(TypeAbi, TopEncode, TopDecode)]
pub struct UserFees<M: ManagedTypeApi> {
    pub fees: EsdtTokenPayment<M>,
    pub epoch: Epoch,
}

#[multiversx_sc::module]
pub trait CommonSubscriberModule {
    fn base_init(&self, fees_contract_address: ManagedAddress) {
        require!(
            self.blockchain().is_smart_contract(&fees_contract_address),
            "Invalid address"
        );

        self.fees_contract_address().set(fees_contract_address);
    }

    /// Arguments are pairs of sc_address, opt_payment_token and payment_amount
    #[only_owner]
    #[endpoint(registerService)]
    fn register_service(
        &self,
        args: MultiValueEncoded<MultiValue3<Option<TokenIdentifier>, BigUint, Epoch>>,
    ) {
        let mut proxy_instance = self.get_subscription_fee_sc_proxy_instance();
        let _: () = proxy_instance
            .register_service(args)
            .execute_on_dest_context();
    }

    #[only_owner]
    #[endpoint(unregisterService)]
    fn unregister_service(&self) {
        let mut proxy_instance = self.get_subscription_fee_sc_proxy_instance();
        let _: () = proxy_instance
            .unregister_service()
            .execute_on_dest_context();
    }

    fn subtract_user_payment(
        &self,
        fees_contract_address: ManagedAddress,
        service_index: usize,
        user_id: AddressId,
    ) {
        let fees_mapper = self.user_fees(service_index, user_id);
        require!(fees_mapper.is_empty(), "User last fees not processed yet");

        let subtract_result =
            self.call_subtract_payment(fees_contract_address, service_index, user_id);
        if let ScResult::Ok(fees) = subtract_result {
            let current_epoch = self.blockchain().get_block_epoch();
            let user_fees = UserFees {
                fees,
                epoch: current_epoch,
            };

            fees_mapper.set(user_fees);
        }
    }

    fn call_subtract_payment(
        &self,
        fee_contract_address: ManagedAddress,
        service_index: usize,
        user_id: AddressId,
    ) -> ScResult<EsdtTokenPayment, ()> {
        self.fee_contract_proxy_obj(fee_contract_address)
            .subtract_payment(service_index, user_id)
            .execute_on_dest_context()
    }

    fn get_subscription_fee_sc_proxy_instance(&self) -> subscription_fee::Proxy<Self::Api> {
        let fees_contract_address = self.fees_contract_address().get();
        self.fee_contract_proxy_obj(fees_contract_address)
    }

    #[proxy]
    fn fee_contract_proxy_obj(
        &self,
        sc_address: ManagedAddress,
    ) -> subscription_fee::Proxy<Self::Api>;

    #[view(getFeesContractAddress)]
    #[storage_mapper("feesContractAddress")]
    fn fees_contract_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[view(getUserFees)]
    #[storage_mapper("userFees")]
    fn user_fees(
        &self,
        service_index: usize,
        user_id: AddressId,
    ) -> SingleValueMapper<UserFees<Self::Api>>;
}
