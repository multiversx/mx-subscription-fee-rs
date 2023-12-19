multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use auto_farm::common::unique_payments::UniquePayments;
use config::ProxyTrait as _;
use subscription_fee::subtract_payments::Epoch;

pub type Percentage = u32;
pub const TOTAL_PERCENTAGE: Percentage = 10_000;

#[derive(TypeAbi, TopEncode, TopDecode, PartialEq)]
pub enum SubscriptionUserType {
    Normal,
    Premium,
}

#[derive(TypeAbi, TopEncode, TopDecode)]
pub struct MexActionsPercentages {
    pub lock: Percentage,
    pub fees: Percentage,
    pub burn: Percentage,
}

pub struct MexActionsValue<M: ManagedTypeApi> {
    pub lock: BigUint<M>,
    pub fees: BigUint<M>,
    pub burn: BigUint<M>,
}

impl MexActionsPercentages {
    pub fn is_valid(&self) -> bool {
        self.lock + self.fees + self.burn == TOTAL_PERCENTAGE
            && self.lock <= TOTAL_PERCENTAGE
            && self.fees <= TOTAL_PERCENTAGE
            && self.burn <= TOTAL_PERCENTAGE
    }

    pub fn get_amounts_per_category<M: ManagedTypeApi>(
        &self,
        total: &BigUint<M>,
    ) -> MexActionsValue<M> {
        let lock_amount = total * self.lock / TOTAL_PERCENTAGE;
        let fees_amount = total * self.fees / TOTAL_PERCENTAGE;
        let burn_amount = total - &lock_amount - &fees_amount;

        MexActionsValue {
            lock: lock_amount,
            fees: fees_amount,
            burn: burn_amount,
        }
    }
}

impl<M: ManagedTypeApi> MexActionsValue<M> {
    pub fn get_sell_amount(&self) -> BigUint<M> {
        &self.lock + &self.burn
    }
}

#[multiversx_sc::module]
pub trait SubscriberConfigModule {
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

    fn claim_boosted_rewards(
        &self,
        sc_address: ManagedAddress,
        user: ManagedAddress,
    ) -> EsdtTokenPayment {
        self.farm_proxy_obj(sc_address)
            .claim_boosted_rewards(user)
            .execute_on_dest_context()
    }

    fn get_user_allow_claim_boosted_rewards(
        &self,
        sc_address: ManagedAddress,
        user: ManagedAddress,
    ) -> bool {
        self.farm_proxy_obj(sc_address)
            .get_allow_external_claim_rewards(user)
            .execute_on_dest_context()
    }

    #[proxy]
    fn other_pair_proxy(&self, sc_address: ManagedAddress) -> pair::Proxy<Self::Api>;

    #[proxy]
    fn simple_lock_proxy(&self, sc_address: ManagedAddress) -> energy_factory::Proxy<Self::Api>;

    #[proxy]
    fn farm_proxy_obj(
        &self,
        sc_address: ManagedAddress,
    ) -> farm_with_locked_rewards::Proxy<Self::Api>;

    #[storage_mapper("mexTokenId")]
    fn mex_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[storage_mapper("wegldTokenId")]
    fn wegld_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[storage_mapper("mexPair")]
    fn mex_pair(&self) -> SingleValueMapper<ManagedAddress>;

    #[view(getSimpleLockAddress)]
    #[storage_mapper("simpleLockAddress")]
    fn simple_lock_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[view(getLockPeriod)]
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

    #[view(getEnergyThreshold)]
    #[storage_mapper("energyThreshold")]
    fn energy_threshold(&self) -> SingleValueMapper<BigUint>;

    #[storage_mapper("farmId")]
    fn farm_id(&self) -> AddressToIdMapper<Self::Api>;

    // used for external storage read
    #[storage_mapper("userId")]
    fn user_id(&self) -> AddressToIdMapper<Self::Api>;
}
