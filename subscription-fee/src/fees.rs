multiversx_sc::imports!();

use auto_farm::common::unique_payments::UniquePayments;

use crate::common_storage;
use crate::pair_actions;

#[multiversx_sc::module]
pub trait FeesModule:
    pair_actions::PairActionsModule + common_storage::CommonStorageModule
{
    #[only_owner]
    #[endpoint(addAcceptedFeesTokens)]
    fn add_accepted_fees_tokens(&self, accepted_tokens: MultiValueEncoded<TokenIdentifier>) {
        for token in accepted_tokens {
            require!(token.is_valid_esdt_identifier(), "Invalid token");

            let _ = self.accepted_fees_tokens().insert(token);
        }
    }

    #[only_owner]
    #[endpoint(setMaxUserDeposits)]
    fn set_max_user_deposits(&self, max_user_deposits: usize) {
        require!(max_user_deposits > 0, "Value must be greater than 0");
        self.max_user_deposits().set(max_user_deposits);
    }

    #[payable("*")]
    #[endpoint]
    fn deposit(&self) {
        let payment = self.call_value().single_esdt();
        require!(payment.amount > 0, "No payment");
        require!(payment.token_nonce == 0, "Can deposit only fungible tokens");
        require!(
            self.accepted_fees_tokens()
                .contains(&payment.token_identifier),
            "Invalid payment token"
        );

        let payment_value_result =
            self.get_price(payment.token_identifier.clone(), payment.amount.clone());
        require!(payment_value_result.is_ok(), "Could not get payment value");

        let payment_value = unsafe { payment_value_result.unwrap_unchecked() };
        let min_user_deposit_value = self.min_user_deposit_value().get();
        require!(
            payment_value > min_user_deposit_value,
            "Payment value is lesser than the minimum accepted"
        );

        let caller = self.blockchain().get_caller();
        let caller_id = self.user_id().get_id_or_insert(&caller);
        self.add_user_payment(payment, self.user_deposited_fees(caller_id));
    }

    #[endpoint(withdrawFunds)]
    fn withdraw_funds(
        &self,
        tokens_to_withdraw: MultiValueEncoded<MultiValue2<TokenIdentifier, BigUint>>,
    ) -> ManagedVec<EsdtTokenPayment> {
        let caller = self.blockchain().get_caller();
        let caller_id = self.user_id().get_id_non_zero(&caller);
        let user_fees_mapper = self.user_deposited_fees(caller_id);
        let mut all_user_tokens = user_fees_mapper.get().into_payments();
        let mut output_payments = ManagedVec::new();
        for pair in tokens_to_withdraw {
            let (token_id, amount) = pair.into_tuple();

            let mut opt_found_token_index = None;
            for (index, user_payment) in all_user_tokens.iter().enumerate() {
                if user_payment.token_identifier == token_id {
                    require!(user_payment.amount >= amount, "User balance not enough");
                    output_payments.push(EsdtTokenPayment::new(token_id, 0, amount.clone()));
                    opt_found_token_index = Some(index);
                    break;
                }
            }

            require!(opt_found_token_index.is_some(), "Payment was not found");

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

        user_fees_mapper.set(&UniquePayments::new_from_unique_payments(all_user_tokens));

        output_payments
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

                let max_user_deposits = self.max_user_deposits().get();
                require!(
                    fees.clone().into_payments().len() < max_user_deposits,
                    "Maximum number of deposits per user reached"
                );
            });
        }
    }
}
