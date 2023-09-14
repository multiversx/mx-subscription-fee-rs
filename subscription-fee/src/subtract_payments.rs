use core::hint::unreachable_unchecked;

use auto_farm::common::{address_to_id_mapper::AddressId, unique_payments::UniquePayments};

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

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
        user_id: AddressId,
        opt_specific_token: Option<EgldOrEsdtTokenIdentifier>,
        amount: BigUint,
    ) -> MyVeryOwnScResult<EgldOrEsdtTokenPayment, ()> {
        let caller = self.blockchain().get_caller();
        let _ = self.service_id().get_id_non_zero(&caller);

        let subtract_result = match opt_specific_token {
            Some(token_id) => self.subtract_specific_token(user_id, token_id, amount),
            None => self.subtract_any_token(user_id, amount),
        };
        if let MyVeryOwnScResult::Ok(token) = &subtract_result {
            self.send().direct(
                &caller,
                &token.token_identifier,
                token.token_nonce,
                &token.amount,
            );
        }

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
                    MyVeryOwnScResult::Err(())
                } else {
                    *egld_value -= &amount;

                    MyVeryOwnScResult::Ok(EgldOrEsdtTokenPayment::new(
                        EgldOrEsdtTokenIdentifier::egld(),
                        0,
                        amount,
                    ))
                }
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
            let mut token = user_tokens.get(i);
            let query_result = self.get_price(token.token_identifier.clone(), amount.clone());
            if query_result.is_err() {
                continue;
            }

            let price = unsafe { query_result.unwrap_unchecked() };
            if price > token.amount {
                continue;
            }

            token.amount -= &price;
            let _ = user_tokens.set(i, &token);
            tokens_mapper.set(UniquePayments::new_from_unique_payments(user_tokens));

            return MyVeryOwnScResult::Ok(EgldOrEsdtTokenPayment::new(
                EgldOrEsdtTokenIdentifier::esdt(token.token_identifier),
                0,
                price,
            ));
        }

        MyVeryOwnScResult::Err(())
    }
}
