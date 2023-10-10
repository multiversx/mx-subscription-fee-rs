use crate::fees;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub mod pair_proxy {
    #[multiversx_sc::proxy]
    pub trait PairProxy {
        #[view(getSafePriceByDefaultOffset)]
        fn get_safe_price_by_default_offset(
            &self,
            pair_address: ManagedAddress,
            input_payment: EsdtTokenPayment,
        ) -> EsdtTokenPayment;
    }
}

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode)]
pub struct PairDataForToken<M: ManagedTypeApi> {
    pub pair_address: ManagedAddress<M>,
    pub other_token_id: TokenIdentifier<M>,
}

#[multiversx_sc::module]
pub trait PairActionsModule: fees::FeesModule {
    #[only_owner]
    #[endpoint(addUsdcPair)]
    fn add_pair_data(
        &self,
        payment_token_id: TokenIdentifier,
        other_token_id: TokenIdentifier,
        pair_address: ManagedAddress,
    ) {
        require!(
            payment_token_id.is_valid_esdt_identifier(),
            "Invalid token ID"
        );
        require!(
            other_token_id.is_valid_esdt_identifier(),
            "Invalid token ID"
        );
        require!(
            self.blockchain().is_smart_contract(&pair_address),
            "Invalid pair address"
        );

        self.pair_data_for_token(&payment_token_id)
            .set(PairDataForToken {
                pair_address,
                other_token_id,
            });
    }

    #[only_owner]
    #[endpoint(removeUsdcPair)]
    fn remove_pair_data(&self, token_id: TokenIdentifier) {
        self.pair_data_for_token(&token_id).clear();
    }

    fn get_price(&self, token_id: TokenIdentifier, amount: BigUint) -> Result<BigUint, ()> {
        let mapper = self.pair_data_for_token(&token_id);
        if mapper.is_empty() {
            return Result::Err(());
        }

        let pair_data = mapper.get();
        let stable_token_id = self.stable_token_id().get();
        let wegld_token_id = self.wegld_token_id().get();
        let price_query_address = self.price_query_address().get();
        let price: EsdtTokenPayment = if token_id == stable_token_id {
            EsdtTokenPayment::new(token_id, 0, amount)
        } else {
            let mut query_payment: EsdtTokenPayment = self
                .pair_proxy(price_query_address.clone())
                .get_safe_price_by_default_offset(
                    pair_data.pair_address,
                    EsdtTokenPayment::new(token_id, 0, amount),
                )
                .execute_on_dest_context();

            if query_payment.token_identifier == wegld_token_id {
                let stable_pair_data_mapper =
                    self.pair_data_for_token(&query_payment.token_identifier);
                if stable_pair_data_mapper.is_empty() {
                    return Result::Err(());
                }
                let stable_pair_data = stable_pair_data_mapper.get();
                query_payment = self
                    .pair_proxy(price_query_address)
                    .get_safe_price_by_default_offset(
                        stable_pair_data.pair_address,
                        EsdtTokenPayment::new(
                            query_payment.token_identifier,
                            0,
                            query_payment.amount,
                        ),
                    )
                    .execute_on_dest_context();
            }

            query_payment
        };

        if price.token_identifier == stable_token_id {
            Result::Ok(price.amount)
        } else {
            Result::Err(())
        }
    }

    #[proxy]
    fn pair_proxy(&self, sc_address: ManagedAddress) -> pair_proxy::Proxy<Self::Api>;

    #[storage_mapper("pairDataForToken")]
    fn pair_data_for_token(
        &self,
        token_id: &TokenIdentifier,
    ) -> SingleValueMapper<PairDataForToken<Self::Api>>;

    #[storage_mapper("priceQueryAddress")]
    fn price_query_address(&self) -> SingleValueMapper<ManagedAddress>;
}
