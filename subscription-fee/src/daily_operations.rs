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
    From<Result<T, E>> for MyVeryOwnScResult<T, E>
{
    fn from(value: Result<T, E>) -> Self {
        match value {
            Result::Ok(t) => MyVeryOwnScResult::Ok(t),
            Result::Err(e) => MyVeryOwnScResult::Err(e),
        }
    }
}

impl<T: NestedEncode + NestedDecode + TypeAbi, E: NestedEncode + NestedDecode + TypeAbi>
    MyVeryOwnScResult<T, E>
{
    pub fn is_err(&self) -> bool {
        matches!(*self, MyVeryOwnScResult::Err(_))
    }
}

#[multiversx_sc::module]
pub trait DailyOperationsModule:
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
    ) -> MyVeryOwnScResult<(), ()> {
        let caller = self.blockchain().get_caller();
        let _ = self.service_id().get_id_non_zero(&caller);

        match opt_specific_token {
            Some(token_id) => {
                if token_id.is_egld() {
                    return self.user_deposited_egld(user_id).update(|egld_value| {
                        if *egld_value < amount {
                            MyVeryOwnScResult::Err(())
                        } else {
                            *egld_value -= amount;

                            MyVeryOwnScResult::Ok(())
                        }
                    });
                }

                let payment =
                    EsdtTokenPayment::new(token_id.clone().unwrap_esdt(), 0, amount.clone());
                let raw_result = self
                    .user_deposited_fees(user_id)
                    .update(|user_fees| user_fees.deduct_payment(&payment));

                raw_result.into()
            }
            None => {
                let tokens_mapper = self.user_deposited_fees(user_id);
                if tokens_mapper.is_empty() {
                    return MyVeryOwnScResult::Err(());
                }

                let mut user_tokens = tokens_mapper.get().into_payments();
                for i in 0..user_tokens.len() {
                    let mut token = user_tokens.get(i);
                    let query_result =
                        self.get_price(token.token_identifier.clone(), amount.clone());
                    if query_result.is_err() {
                        continue;
                    }

                    let price = unsafe { query_result.unwrap_unchecked() };
                    if price > token.amount {
                        continue;
                    }

                    token.amount -= price;
                    let _ = user_tokens.set(i, &token);
                    tokens_mapper.set(UniquePayments::new_from_unique_payments(user_tokens));

                    return MyVeryOwnScResult::Ok(());
                }

                MyVeryOwnScResult::Err(())
            }
        }
    }
}
