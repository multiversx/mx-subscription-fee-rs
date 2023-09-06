use core::ops::Deref;

use auto_farm::common::{address_to_id_mapper::AddressId, unique_payments::UniquePayments};

use crate::service::ServiceInfo;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[derive(TypeAbi, TopEncode, TopDecode)]
pub struct InterpretedResult<M: ManagedTypeApi> {
    pub opt_new_token: Option<EsdtTokenPayment<M>>,
    pub user_rewards: ManagedVec<M, EsdtTokenPayment<M>>,
}

#[multiversx_sc::module]
pub trait LowLevelActionsModule:
    crate::fees::FeesModule + crate::user_tokens::UserTokensModule + crate::service::ServiceModule
{
    fn perform_action(
        &self,
        user_address: ManagedAddress,
        user_id: AddressId,
        service_info: &ServiceInfo<Self::Api>,
    ) -> Result<InterpretedResult<Self::Api>, ()> {
        let raw_results: MultiValueEncoded<ManagedBuffer> = match &service_info.opt_endpoint_payment
        {
            Some(token_id) => {
                let search_result = self.find_token(user_id, token_id);
                if search_result.is_err() {
                    return Result::Err(());
                }

                let found_token = unsafe { search_result.unwrap_unchecked() };
                let mut contract_call = ContractCallWithEgldOrSingleEsdt::<
                    Self::Api,
                    MultiValueEncoded<ManagedBuffer>,
                >::new(
                    service_info.sc_address.clone(),
                    service_info.endpoint_name.clone(),
                    EgldOrEsdtTokenIdentifier::esdt(found_token.token_identifier),
                    found_token.token_nonce,
                    found_token.amount,
                );
                contract_call.push_raw_argument(user_address.as_managed_buffer().clone()); // original caller arg

                contract_call.execute_on_dest_context()
            }
            None => {
                let mut contract_call =
                    ContractCallNoPayment::<Self::Api, MultiValueEncoded<ManagedBuffer>>::new(
                        service_info.sc_address.clone(),
                        service_info.endpoint_name.clone(),
                    );
                contract_call.push_raw_argument(user_address.as_managed_buffer().clone()); // original caller arg

                contract_call.execute_on_dest_context()
            }
        };

        let interpreted_results = self.interpret_results(
            service_info.sc_address.clone(),
            raw_results,
            service_info.opt_interpret_results_endpoint.clone(),
        );

        Result::Ok(interpreted_results)
    }

    fn find_token(
        &self,
        user_id: AddressId,
        token_id: &TokenIdentifier,
    ) -> Result<EsdtTokenPayment, ()> {
        let mapper = self.user_deposited_tokens(user_id);
        let mut user_tokens = mapper.get().into_payments();
        for i in 0..user_tokens.len() {
            let token = user_tokens.get(i);
            if &token.token_identifier != token_id {
                continue;
            }

            user_tokens.remove(i);
            mapper.set(UniquePayments::new_from_unique_payments(user_tokens));

            return Result::Ok(token);
        }

        Result::Err(())
    }

    fn interpret_results(
        &self,
        sc_address: ManagedAddress,
        raw_results: MultiValueEncoded<ManagedBuffer>,
        opt_func: Option<ManagedBuffer>,
    ) -> InterpretedResult<Self::Api> {
        match opt_func {
            Some(func) => {
                let mut contract_call = ContractCallNoPayment::<
                    Self::Api,
                    MultiValueEncoded<ManagedBuffer>,
                >::new(sc_address, func);
                for buffer in raw_results {
                    contract_call.push_raw_argument(buffer);
                }
                let interpreted_result: InterpretedResult<Self::Api> =
                    contract_call.execute_on_dest_context();

                interpreted_result
            }
            None => {
                let results_vec = raw_results.to_vec();
                InterpretedResult {
                    opt_new_token: Some(
                        EsdtTokenPayment::top_decode(results_vec.get(0).deref().clone()).unwrap(),
                    ),
                    user_rewards: ManagedVec::top_decode(results_vec.get(1).deref().clone())
                        .unwrap(),
                }
            }
        }
    }
}
