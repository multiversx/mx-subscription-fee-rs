use core::hint::unreachable_unchecked;

use auto_farm::common::{address_to_id_mapper::AddressId, unique_payments::UniquePayments};

pub type Epoch = u64;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

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

        let last_action_mapper = self.user_last_action_epoch(user_id, service_id, service_index);
        let last_action_epoch = last_action_mapper.get();

        let service_info = self.service_info(service_id).get().get(service_index);

        let subscription_epochs = service_info.subscription_epochs;

        if subscription_epochs == 0 {
            return ScResult::Err(());
        }

        let next_subtract_epoch = if last_action_epoch > 0 {
            last_action_epoch + subscription_epochs
        } else {
            current_epoch
        };
        require!(next_subtract_epoch <= current_epoch, "Cannot subtract yet");

        let opt_user_address = self.user_id().get_address(user_id);
        if opt_user_address.is_none() {
            return ScResult::Err(());
        }

        let subtract_result = match service_info.opt_payment_token {
            Some(token_id) => self.subtract_specific_token(user_id, token_id, service_info.amount),
            None => self.subtract_any_token(user_id, service_info.amount),
        };
        if let ScResult::Ok(payment) = &subtract_result {
            self.send().direct_esdt(
                &caller,
                &payment.token_identifier,
                payment.token_nonce,
                &payment.amount,
            );

            last_action_mapper.set(next_subtract_epoch);
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

    fn subtract_any_token(
        &self,
        user_id: AddressId,
        amount_in_stable_token: BigUint,
    ) -> ScResult<EsdtTokenPayment, ()> {
        let tokens_mapper = self.user_deposited_fees(user_id);
        if tokens_mapper.is_empty() {
            return ScResult::Err(());
        }

        let mut user_tokens = tokens_mapper.get().into_payments();
        for i in 0..user_tokens.len() {
            let mut payment = user_tokens.get(i);
            let query_result =
                self.get_price(payment.token_identifier.clone(), payment.amount.clone());
            if query_result.is_err() {
                continue;
            }

            let price = unsafe { query_result.unwrap_unchecked() };
            // TODO
            // Think about progressive deduction
            if price < amount_in_stable_token {
                continue;
            }

            let tokens_to_pay = &payment.amount * &amount_in_stable_token / price;

            payment.amount -= &tokens_to_pay;
            let _ = user_tokens.set(i, &payment);
            tokens_mapper.set(UniquePayments::new_from_unique_payments(user_tokens));

            return ScResult::Ok(EsdtTokenPayment::new(
                payment.token_identifier,
                0,
                tokens_to_pay,
            ));
        }

        ScResult::Err(())
    }

    #[storage_mapper("userLastActionEpoch")]
    fn user_last_action_epoch(
        &self,
        user_id: AddressId,
        service_id: AddressId,
        service_index: usize,
    ) -> SingleValueMapper<Epoch>;
}
