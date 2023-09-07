use auto_farm::common::{address_to_id_mapper::AddressId, unique_payments::UniquePayments};

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait UserTokensModule: crate::fees::FeesModule {
    #[only_owner]
    #[endpoint(addAcceptedUserTokens)]
    fn add_accepted_user_tokens(&self, accepted_tokens: MultiValueEncoded<TokenIdentifier>) {
        for token in accepted_tokens {
            require!(token.is_valid_esdt_identifier(), "Invalid token");

            self.accepted_user_tokens().insert(token);
        }
    }

    #[payable("*")]
    #[endpoint(depositTokens)]
    fn deposit_tokens(&self) {
        let payment = self.call_value().single_esdt();
        require!(payment.amount > 0, "No payment");
        require!(
            self.accepted_user_tokens()
                .contains(&payment.token_identifier),
            "Invalid payment token"
        );

        let caller = self.blockchain().get_caller();
        let caller_id = self.user_id().get_id_or_insert(&caller);
        self.add_user_payment(
            caller_id,
            EgldOrEsdtTokenPayment::from(payment),
            self.user_deposited_tokens(caller_id),
        );
    }

    #[endpoint(withdrawTokens)]
    fn withdraw_tokens(&self) {}

    #[view(getAcceptedUserTokens)]
    #[storage_mapper("acceptedUserTokens")]
    fn accepted_user_tokens(&self) -> UnorderedSetMapper<TokenIdentifier>;

    #[storage_mapper("userDepositedTokens")]
    fn user_deposited_tokens(
        &self,
        user_id: AddressId,
    ) -> SingleValueMapper<UniquePayments<Self::Api>>;
}
