multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use core::hint::unreachable_unchecked;

pub type Epoch = u64;

#[must_use]
#[derive(Debug, PartialEq, Eq, Clone, TopEncode, TopDecode, TypeAbi)]
pub enum ScResult<
    T: NestedEncode + NestedDecode + TypeAbi,
    E: NestedEncode + NestedDecode + TypeAbi,
> {
    Ok(T),
    Err(E),
}

impl<T: NestedEncode + NestedDecode + TypeAbi, E: NestedEncode + NestedDecode + TypeAbi>
    ScResult<T, E>
{
    pub fn is_err(&self) -> bool {
        matches!(*self, ScResult::Err(_))
    }

    /// # Safety
    ///
    /// Calling this method on an [`Err`] is *[undefined behavior]*.
    pub unsafe fn unwrap_unchecked(self) -> T {
        match self {
            ScResult::Ok(t) => t,
            ScResult::Err(_) => unreachable_unchecked(),
        }
    }
}

#[multiversx_sc::module]
pub trait SubtractPaymentsModule:
    crate::fees::FeesModule
    + crate::service::ServiceModule
    + crate::pair_actions::PairActionsModule
    + crate::common_storage::CommonStorageModule
    + multiversx_sc_modules::ongoing_operation::OngoingOperationModule
{
    #[endpoint(subtractPayment)]
    fn subtract_payment(
        &self,
        service_index: usize,
        user_id: AddressId,
    ) -> ScResult<EsdtTokenPayment, ()> {
        let caller = self.blockchain().get_caller();
        let service_id = self.service_id().get_id_non_zero(&caller);
        let current_epoch = self.blockchain().get_block_epoch();

        if !self
            .subscribed_users(service_id, service_index)
            .contains(&user_id)
        {
            return ScResult::Err(());
        }

        let next_payment_mapper = self.user_next_payment_epoch(user_id, service_id, service_index);
        let next_payment_epoch = next_payment_mapper.get();
        require!(next_payment_epoch <= current_epoch, "Cannot subtract yet");

        let service_info = self.service_info(service_id).get().get(service_index);
        let subscription_epochs = service_info.subscription_epochs;

        if subscription_epochs == 0 {
            return ScResult::Err(());
        }

        let opt_user_address = self.user_id().get_address(user_id);
        if opt_user_address.is_none() {
            return ScResult::Err(());
        }

        let subtract_result = match service_info.opt_payment_token {
            Some(token_id) => {
                if service_info.payment_in_stable {
                    self.subtract_specific_token_in_stable(user_id, token_id, service_info.amount)
                } else {
                    self.subtract_specific_token(user_id, token_id, service_info.amount)
                }
            }
            None => self.subtract_any_token(user_id, service_info.amount),
        };
        if let ScResult::Ok(payment) = &subtract_result {
            self.send().direct_esdt(
                &caller,
                &payment.token_identifier,
                payment.token_nonce,
                &payment.amount,
            );

            next_payment_mapper.set(current_epoch + subscription_epochs);
        }

        subtract_result
    }

    fn subtract_specific_token(
        &self,
        user_id: AddressId,
        token_id: TokenIdentifier,
        amount: BigUint,
    ) -> ScResult<EsdtTokenPayment, ()> {
        let payment = EsdtTokenPayment::new(token_id, 0, amount);
        let raw_result = self
            .user_deposited_fees(user_id)
            .update(|user_fees| user_fees.deduct_payment(&payment));

        match raw_result {
            Result::Ok(()) => ScResult::Ok(payment),
            Result::Err(()) => ScResult::Err(()),
        }
    }

    fn subtract_specific_token_in_stable(
        &self,
        user_id: AddressId,
        token_id: TokenIdentifier,
        amount_in_stable_token: BigUint,
    ) -> ScResult<EsdtTokenPayment, ()> {
        let query_result = self.get_worth_of_price(&token_id, amount_in_stable_token);
        if query_result.is_err() {
            return ScResult::Err(());
        }

        let tokens_to_pay = unsafe { query_result.unwrap_unchecked() };
        let payment_to_deduct = EsdtTokenPayment::new(token_id, 0, tokens_to_pay);
        let raw_result = self
            .user_deposited_fees(user_id)
            .update(|user_fees| user_fees.deduct_payment(&payment_to_deduct));

        match raw_result {
            Result::Ok(()) => ScResult::Ok(payment_to_deduct),
            Result::Err(()) => ScResult::Err(()),
        }
    }

    fn subtract_any_token(
        &self,
        user_id: AddressId,
        amount_in_stable_token: BigUint,
    ) -> ScResult<EsdtTokenPayment, ()> {
        let tokens_mapper = self.user_deposited_fees(user_id);
        if tokens_mapper.is_empty() {
            return ScResult::Err(());
        }

        let user_tokens = tokens_mapper.get().into_payments();
        for user_token in user_tokens.iter() {
            let subtract_result = self.subtract_specific_token_in_stable(
                user_id,
                user_token.token_identifier,
                amount_in_stable_token.clone(),
            );

            if !subtract_result.is_err() {
                return subtract_result;
            }
        }

        ScResult::Err(())
    }
}
