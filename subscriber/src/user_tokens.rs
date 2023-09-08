use auto_farm::common::{address_to_id_mapper::AddressId, unique_payments::UniquePayments};

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait UserTokensModule: crate::common_storage::CommonStorageModule {
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

        let fees_addresss = self.fees_contract_address().get();
        let caller = self.blockchain().get_caller();
        let caller_id = self
            .user_id()
            .get_id_at_address_non_zero(&fees_addresss, &caller);
        self.add_user_payment(payment, self.user_deposited_tokens(caller_id));
    }

    fn add_user_payment(
        &self,
        payment: EsdtTokenPayment,
        dest_mapper: SingleValueMapper<UniquePayments<Self::Api>>,
    ) {
        if dest_mapper.is_empty() {
            let user_fees = UniquePayments::<Self::Api>::new_from_unique_payments(
                ManagedVec::from_single_item(payment),
            );

            dest_mapper.set(&user_fees);
        } else {
            dest_mapper.update(|fees| {
                fees.add_payment(payment);
            });
        }
    }

    #[endpoint(withdrawTokens)]
    fn withdraw_tokens(
        &self,
        tokens_to_withdraw: MultiValueEncoded<MultiValue3<TokenIdentifier, u64, BigUint>>,
    ) -> ManagedVec<EsdtTokenPayment> {
        let fees_addresss = self.fees_contract_address().get();
        let caller = self.blockchain().get_caller();
        let caller_id = self
            .user_id()
            .get_id_at_address_non_zero(&fees_addresss, &caller);

        let user_tokens_mapper = self.user_deposited_tokens(caller_id);
        let mut all_user_tokens = user_tokens_mapper.get().into_payments();
        let mut output_payments = ManagedVec::new();
        for pair in tokens_to_withdraw {
            let (token_id, nonce, amount) = pair.into_tuple();

            let mut opt_found_token_index = None;
            for (index, user_token) in all_user_tokens.iter().enumerate() {
                if user_token.token_identifier == token_id
                    && user_token.token_nonce == nonce
                    && user_token.amount >= amount
                {
                    output_payments.push(user_token);
                    opt_found_token_index = Some(index);
                    break;
                }
            }

            if opt_found_token_index.is_none() {
                continue;
            }

            let token_index = unsafe { opt_found_token_index.unwrap_unchecked() };
            let mut token_info = all_user_tokens.get(token_index);
            if token_info.amount == amount {
                all_user_tokens.remove(token_index);
            } else {
                token_info.amount -= amount;
                let _ = all_user_tokens.set(token_index, &token_info);
            }
        }

        if !output_payments.is_empty() {
            self.send().direct_multi(&caller, &output_payments);
        }

        user_tokens_mapper.set(&UniquePayments::new_from_unique_payments(all_user_tokens));

        output_payments
    }

    #[view(getAcceptedUserTokens)]
    #[storage_mapper("acceptedUserTokens")]
    fn accepted_user_tokens(&self) -> UnorderedSetMapper<TokenIdentifier>;

    #[storage_mapper("userDepositedTokens")]
    fn user_deposited_tokens(
        &self,
        user_id: AddressId,
    ) -> SingleValueMapper<UniquePayments<Self::Api>>;
}
