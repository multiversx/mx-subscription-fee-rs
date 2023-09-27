use core::hint::unreachable_unchecked;

use auto_farm::common::{address_to_id_mapper::AddressId, unique_payments::UniquePayments};

use crate::service::SubscriptionType;

pub type Epoch = u64;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub const DAILY_EPOCHS: Epoch = 1;
pub const WEEKLY_EPOCHS: Epoch = 7;
pub const MONTHLY_EPOCHS: Epoch = 30;

#[must_use]
#[derive(Debug, PartialEq, Eq, Clone, TopEncode, TopDecode, TypeAbi)]
pub enum MyVeryOwnScResult<
    T: NestedEncode + NestedDecode + TypeAbi,
    E: NestedEncode + NestedDecode + TypeAbi,
> {
    Ok(T),
    Err(E),
}

impl<T: NestedEncode + NestedDecode + TypeAbi, E: NestedEncode + NestedDecode + TypeAbi>
    MyVeryOwnScResult<T, E>
{
    pub fn is_err(&self) -> bool {
        matches!(*self, MyVeryOwnScResult::Err(_))
    }

    /// # Safety
    ///
    /// Calling this method on an [`Err`] is *[undefined behavior]*.
    pub unsafe fn unwrap_unchecked(self) -> T {
        match self {
            MyVeryOwnScResult::Ok(t) => t,
            MyVeryOwnScResult::Err(_) => unreachable_unchecked(),
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
    ) -> MyVeryOwnScResult<EgldOrEsdtTokenPayment, ()> {
        let caller = self.blockchain().get_caller();
        let service_id = self.service_id().get_id_non_zero(&caller);
        let current_epoch = self.blockchain().get_block_epoch();

        let last_action_mapper = self.user_last_action_epoch(user_id, service_id, service_index);
        let last_action_epoch = last_action_mapper.get();
        if last_action_epoch > 0 {
            let next_subtract_epoch = last_action_epoch + MONTHLY_EPOCHS;
            require!(next_subtract_epoch <= current_epoch, "Cannot subtract yet");
        }

        let opt_user_address = self.user_id().get_address(user_id);
        if opt_user_address.is_none() {
            return MyVeryOwnScResult::Err(());
        }

        let subscription_type = self
            .subscription_type(user_id, service_id, service_index)
            .get();
        let multiplier = match subscription_type {
            SubscriptionType::Daily => MONTHLY_EPOCHS / DAILY_EPOCHS,
            SubscriptionType::Weekly => MONTHLY_EPOCHS / WEEKLY_EPOCHS,
            SubscriptionType::Monthly => 1,
            SubscriptionType::None => return MyVeryOwnScResult::Err(()),
        };

        let service_info = self.service_info(service_id).get().get(service_index);
        let subtract_result = match service_info.opt_payment_token {
            Some(token_id) => {
                self.subtract_specific_token(user_id, token_id, service_info.amount * multiplier)
            }
            None => self.subtract_any_token(user_id, service_info.amount * multiplier),
        };
        if let MyVeryOwnScResult::Ok(payment) = &subtract_result {
            self.send().direct(
                &caller,
                &payment.token_identifier,
                payment.token_nonce,
                &payment.amount,
            );
        }

        last_action_mapper.set(current_epoch);

        subtract_result
    }

    fn subtract_specific_token(
        &self,
        user_id: AddressId,
        token_id: EgldOrEsdtTokenIdentifier,
        amount: BigUint,
    ) -> MyVeryOwnScResult<EgldOrEsdtTokenPayment, ()> {
        if token_id.is_egld() {
            return self.user_deposited_egld(user_id).update(|egld_value| {
                if *egld_value < amount {
                    return MyVeryOwnScResult::Err(());
                }

                *egld_value -= &amount;

                MyVeryOwnScResult::Ok(EgldOrEsdtTokenPayment::new(
                    EgldOrEsdtTokenIdentifier::egld(),
                    0,
                    amount,
                ))
            });
        }

        let payment = EsdtTokenPayment::new(token_id.unwrap_esdt(), 0, amount);
        let raw_result = self
            .user_deposited_fees(user_id)
            .update(|user_fees| user_fees.deduct_payment(&payment));

        match raw_result {
            Result::Ok(()) => MyVeryOwnScResult::Ok(payment.into()),
            Result::Err(()) => MyVeryOwnScResult::Err(()),
        }
    }

    fn subtract_any_token(
        &self,
        user_id: AddressId,
        amount: BigUint,
    ) -> MyVeryOwnScResult<EgldOrEsdtTokenPayment, ()> {
        let tokens_mapper = self.user_deposited_fees(user_id);
        if tokens_mapper.is_empty() {
            return MyVeryOwnScResult::Err(());
        }

        let mut user_tokens = tokens_mapper.get().into_payments();
        for i in 0..user_tokens.len() {
            let mut payment = user_tokens.get(i);
            let query_result = self.get_price(payment.token_identifier.clone(), amount.clone());
            if query_result.is_err() {
                continue;
            }

            let price = unsafe { query_result.unwrap_unchecked() };
            if price > payment.amount {
                continue;
            }

            payment.amount -= &price;
            let _ = user_tokens.set(i, &payment);
            tokens_mapper.set(UniquePayments::new_from_unique_payments(user_tokens));

            return MyVeryOwnScResult::Ok(EgldOrEsdtTokenPayment::new(
                EgldOrEsdtTokenIdentifier::esdt(payment.token_identifier),
                0,
                price,
            ));
        }

        MyVeryOwnScResult::Err(())
    }

    #[storage_mapper("userLastActionEpoch")]
    fn user_last_action_epoch(
        &self,
        user_id: AddressId,
        service_id: AddressId,
        service_index: usize,
    ) -> SingleValueMapper<Epoch>;
}
