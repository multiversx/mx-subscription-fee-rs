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

#[multiversx_sc::module]
pub trait PairActionsModule: crate::common_storage::CommonStorageModule {
    #[only_owner]
    #[endpoint(addPairAddress)]
    fn add_pair_address(&self, payment_token_id: TokenIdentifier, pair_address: ManagedAddress) {
        require!(
            payment_token_id.is_valid_esdt_identifier(),
            "Invalid token ID"
        );
        require!(
            self.blockchain().is_smart_contract(&pair_address),
            "Invalid pair address"
        );

        self.pair_address_for_token(&payment_token_id)
            .set(pair_address);
    }

    #[only_owner]
    #[endpoint(removePairAddress)]
    fn remove_pair_address(&self, token_id: TokenIdentifier) {
        self.pair_address_for_token(&token_id).clear();
    }

    fn get_worth_of_price(
        &self,
        desired_token_id: &TokenIdentifier,
        stable_worth_amount: BigUint,
    ) -> Result<BigUint, ()> {
        let stable_token_id = self.stable_token_id().get();
        if desired_token_id == &stable_token_id {
            return Result::Ok(stable_worth_amount);
        }

        let wegld_token_id = self.wegld_token_id().get();
        let price_query_address = self.price_query_address().get();
        let stable_pair_data_mapper = self.pair_address_for_token(&wegld_token_id);
        if stable_pair_data_mapper.is_empty() {
            return Result::Err(());
        }
        let stable_pair_address = stable_pair_data_mapper.get();
        let wegld_price: EsdtTokenPayment = self
            .pair_proxy(price_query_address.clone())
            .get_safe_price_by_default_offset(
                stable_pair_address,
                EsdtTokenPayment::new(stable_token_id, 0, stable_worth_amount),
            )
            .execute_on_dest_context();

        if desired_token_id == &wegld_price.token_identifier {
            return Result::Ok(wegld_price.amount);
        }

        let token_mapper = self.pair_address_for_token(desired_token_id);
        if token_mapper.is_empty() {
            return Result::Err(());
        }

        let pair_address = token_mapper.get();
        let price: EsdtTokenPayment = self
            .pair_proxy(price_query_address)
            .get_safe_price_by_default_offset(pair_address, wegld_price)
            .execute_on_dest_context();

        if &price.token_identifier == desired_token_id {
            Result::Ok(price.amount)
        } else {
            Result::Err(())
        }
    }

    #[proxy]
    fn pair_proxy(&self, sc_address: ManagedAddress) -> pair_proxy::Proxy<Self::Api>;
}
