use auto_farm::common::unique_payments::UniquePayments;
use subscription_fee::subtract_payments::Epoch;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub type Percentage = u32;
pub const TOTAL_PERCENTAGE: Percentage = 10_000;

#[derive(TypeAbi, TopEncode, TopDecode)]
pub struct MexActionsPercentages {
    pub lock: Percentage,
    pub fees: Percentage,
    pub mex_burn: Percentage,
}

pub struct MexActionsValue<M: ManagedTypeApi> {
    pub lock: BigUint<M>,
    pub fees: BigUint<M>,
    pub mex_burn: BigUint<M>,
}

impl MexActionsPercentages {
    pub fn is_valid(&self) -> bool {
        self.lock + self.fees + self.mex_burn == TOTAL_PERCENTAGE
    }

    pub fn get_amounts_per_category<M: ManagedTypeApi>(
        &self,
        total: &BigUint<M>,
    ) -> MexActionsValue<M> {
        let lock_amount = total * self.lock / TOTAL_PERCENTAGE;
        let fees_amount = total * self.fees / TOTAL_PERCENTAGE;
        let mex_burn_amount = total - &lock_amount - &fees_amount;

        MexActionsValue {
            lock: lock_amount,
            fees: fees_amount,
            mex_burn: mex_burn_amount,
        }
    }
}

impl<M: ManagedTypeApi> MexActionsValue<M> {
    pub fn get_total_mex(&self) -> BigUint<M> {
        &self.lock + &self.mex_burn
    }
}

#[multiversx_sc::module]
pub trait BuyMexModule {
    #[only_owner]
    #[endpoint(addMexPair)]
    fn add_mex_pair(&self, token_id: TokenIdentifier, pair_address: ManagedAddress) {
        require!(token_id.is_valid_esdt_identifier(), "Invalid token ID");
        require!(
            self.blockchain().is_smart_contract(&pair_address),
            "Invalid pair address"
        );

        self.mex_pairs(&token_id).set(pair_address);
    }

    #[only_owner]
    #[endpoint(removeMexPair)]
    fn remove_mex_pair(&self, token_id: TokenIdentifier) {
        self.mex_pairs(&token_id).clear();
    }

    fn perform_mex_operations(
        &self,
        user_address: ManagedAddress,
        token_id: TokenIdentifier,
        total_tokens: BigUint,
        actions_percentages: &MexActionsPercentages,
    ) {
        let actions_value = actions_percentages.get_amounts_per_category(&total_tokens);
        let total_mex_to_buy = actions_value.get_total_mex();

        if actions_value.fees > 0 {
            self.total_fees().update(|fees| {
                fees.add_payment(EsdtTokenPayment::new(
                    token_id.clone(),
                    0,
                    actions_value.fees.clone(),
                ))
            });
        }

        let opt_bought_mex = self.buy_mex(token_id.clone(), total_mex_to_buy.clone());
        if opt_bought_mex.is_none() {
            // TODO: Maybe do something else with it? Discuss.
            self.total_fees().update(|fees| {
                fees.add_payment(EsdtTokenPayment::new(token_id, 0, total_mex_to_buy))
            });

            return;
        }

        let bought_mex = unsafe { opt_bought_mex.unwrap_unchecked() };
        let mex_to_lock = &bought_mex.amount * actions_percentages.lock / TOTAL_PERCENTAGE;
        let mex_to_burn = bought_mex.amount - &mex_to_lock;

        if mex_to_burn > 0 {
            self.send()
                .esdt_local_burn(&bought_mex.token_identifier, 0, &mex_to_burn);
        }

        if mex_to_lock == 0 {
            return;
        }

        let simple_lock_address = self.simple_lock_address().get();
        let lock_period = self.lock_period().get();
        let _ = self.call_lock_tokens(
            simple_lock_address,
            EsdtTokenPayment::new(bought_mex.token_identifier, 0, mex_to_lock),
            lock_period,
            user_address,
        );
    }

    fn buy_mex(&self, token_id: TokenIdentifier, amount: BigUint) -> Option<EsdtTokenPayment> {
        if amount == 0 {
            return None;
        }

        let pair_mapper = self.mex_pairs(&token_id);
        if pair_mapper.is_empty() {
            return None;
        }

        let mex_token_id = self.mex_token_id().get();
        let pair_address = pair_mapper.get();
        let mex_tokens = self.call_swap_to_mex(pair_address, mex_token_id, token_id, amount);

        Some(mex_tokens)
    }

    fn call_swap_to_mex(
        &self,
        pair_address: ManagedAddress,
        mex_token_id: TokenIdentifier,
        input_token_id: TokenIdentifier,
        amount: BigUint,
    ) -> EsdtTokenPayment {
        self.other_pair_proxy(pair_address)
            .swap_tokens_fixed_input(mex_token_id, BigUint::from(1u32))
            .with_esdt_transfer(EsdtTokenPayment::new(input_token_id, 0, amount))
            .execute_on_dest_context()
    }

    fn call_lock_tokens(
        &self,
        simple_lock_address: ManagedAddress,
        input_tokens: EsdtTokenPayment,
        lock_epochs: Epoch,
        destination: ManagedAddress,
    ) -> EsdtTokenPayment {
        self.simple_lock_proxy(simple_lock_address)
            .lock_tokens_endpoint(lock_epochs, destination)
            .with_esdt_transfer(input_tokens)
            .execute_on_dest_context()
    }

    #[proxy]
    fn other_pair_proxy(&self, sc_address: ManagedAddress) -> pair::Proxy<Self::Api>;

    #[proxy]
    fn simple_lock_proxy(&self, sc_address: ManagedAddress) -> energy_factory::Proxy<Self::Api>;

    #[storage_mapper("mexTokenId")]
    fn mex_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[storage_mapper("mexPairs")]
    fn mex_pairs(&self, token_id: &TokenIdentifier) -> SingleValueMapper<ManagedAddress>;

    #[storage_mapper("simpleLockAddress")]
    fn simple_lock_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[storage_mapper("lockPeriod")]
    fn lock_period(&self) -> SingleValueMapper<Epoch>;

    #[view(getNormalUserPercentage)]
    #[storage_mapper("normalUserPercentage")]
    fn normal_user_percentage(&self) -> SingleValueMapper<MexActionsPercentages>;

    #[view(getPremiumUserPercentage)]
    #[storage_mapper("premiumUserPercentage")]
    fn premium_user_percentage(&self) -> SingleValueMapper<MexActionsPercentages>;

    #[view(getTotalFees)]
    #[storage_mapper("totalFees")]
    fn total_fees(&self) -> SingleValueMapper<UniquePayments<Self::Api>>;
}
