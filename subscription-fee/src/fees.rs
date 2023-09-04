use auto_farm::common::{
    address_to_id_mapper::{AddressId, AddressToIdMapper, NULL_ID},
    unique_payments::UniquePayments,
};

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait FeesModule {
    #[only_owner]
    #[endpoint(addAcceptedFeesTokens)]
    fn add_accepted_fees_tokens(
        &self,
        accepted_tokens: MultiValueEncoded<EgldOrEsdtTokenIdentifier>,
    ) {
        for token in accepted_tokens {
            require!(token.is_valid(), "Invalid token");

            self.accepted_fees_tokens().insert(token);
        }
    }

    #[payable("*")]
    #[endpoint(depositFees)]
    fn deposit_fees(&self) {
        let (payment_token, payment_amount) = self.call_value().egld_or_single_fungible_esdt();
        require!(payment_amount > 0, "No payment");
        require!(
            self.accepted_fees_tokens().contains(&payment_token),
            "Invalid payment token"
        );

        let caller = self.blockchain().get_caller();
        let caller_id = self.user_ids().get_id_or_insert(&caller);
        self.add_user_payment(caller_id, payment_token, payment_amount);
    }

    #[endpoint(withdrawFunds)]
    fn withdraw_funds(
        &self,
        tokens_to_withdraw: MultiValueEncoded<EgldOrEsdtTokenIdentifier>,
    ) -> MultiValue2<BigUint, ManagedVec<EsdtTokenPayment>> {
        let caller = self.blockchain().get_caller();
        let caller_id = self.user_ids().get_id_or_insert(&caller);
        require!(caller_id != NULL_ID, "Unknown user");

        let tokens = if tokens_to_withdraw.is_empty() {
            let mapper = self.accepted_fees_tokens();
            let total_tokens = mapper.len();
            let mut all_tokens = ManagedVec::new();
            for i in 1..=total_tokens {
                let token_at_index = mapper.get_by_index(i);
                all_tokens.push(token_at_index);
            }

            all_tokens
        } else {
            tokens_to_withdraw.to_vec()
        };

        let user_fees_mapper = self.user_deposited_fees(caller_id);
        let mut all_user_tokens = user_fees_mapper.get().into_payments();
        let mut egld_amount = BigUint::zero();
        let mut output_payments = ManagedVec::new();
        for token in &tokens {
            if token.is_egld() {
                let user_egld_amount = self.user_deposited_egld(caller_id).take();
                if user_egld_amount > 0 {
                    self.send().direct_egld(&caller, &user_egld_amount);
                    egld_amount = user_egld_amount;
                }

                continue;
            }

            let token_id = token.unwrap_esdt();
            let mut opt_found_token_index = None;
            for (index, user_token) in all_user_tokens.iter().enumerate() {
                if user_token.token_identifier == token_id && user_token.amount > 0 {
                    output_payments.push(user_token);
                    opt_found_token_index = Some(index);
                    break;
                }
            }

            if let Some(index) = opt_found_token_index {
                all_user_tokens.remove(index)
            }
        }

        if !output_payments.is_empty() {
            self.send().direct_multi(&caller, &output_payments);
        }

        user_fees_mapper.set(&UniquePayments::new_from_unique_payments(all_user_tokens));

        (egld_amount, output_payments).into()
    }

    fn add_user_payment(
        &self,
        caller_id: AddressId,
        payment_token: EgldOrEsdtTokenIdentifier,
        payment_amount: BigUint,
    ) {
        if payment_token.is_egld() {
            self.user_deposited_egld(caller_id)
                .update(|deposited_egld| *deposited_egld += payment_amount);

            return;
        }

        let fees_mapper = self.user_deposited_fees(caller_id);
        if fees_mapper.is_empty() {
            let payment = EsdtTokenPayment::new(payment_token.unwrap_esdt(), 0, payment_amount);
            let user_fees = UniquePayments::<Self::Api>::new_from_unique_payments(
                ManagedVec::from_single_item(payment),
            );

            fees_mapper.set(&user_fees);
        } else {
            let payment = EsdtTokenPayment::new(payment_token.unwrap_esdt(), 0, payment_amount);
            fees_mapper.update(|fees| {
                fees.add_payment(payment);
            });
        }
    }

    // test

    #[view(getAcceptedFeesTokens)]
    #[storage_mapper("acceptedFeesTokens")]
    fn accepted_fees_tokens(&self) -> UnorderedSetMapper<EgldOrEsdtTokenIdentifier>;

    #[storage_mapper("userToIdMapper")]
    fn user_ids(&self) -> AddressToIdMapper<Self::Api>;

    #[storage_mapper("userDepositedFees")]
    fn user_deposited_fees(
        &self,
        user_id: AddressId,
    ) -> SingleValueMapper<UniquePayments<Self::Api>>;

    #[storage_mapper("userDepositedEgld")]
    fn user_deposited_egld(&self, user_id: AddressId) -> SingleValueMapper<BigUint>;
}
