multiversx_sc::imports!();
multiversx_sc::derive_imports!();

mod claim_farm_boosted_rewards_proxy {
    multiversx_sc::imports!();

    #[multiversx_sc::proxy]
    pub trait ClaimFarmBoostedRewardsProxy {
        #[endpoint(claimBoostedRewards)]
        fn claim_boosted_rewards(&self, user: ManagedAddress) -> EsdtTokenPayment<Self::Api>;
    }
}

#[derive(ManagedVecItem, Clone)]
pub struct AdditionalFarmData {
    pub dummy_data: u8,
}

#[multiversx_sc::module]
pub trait ClaimFarmBoostedRewardsModule {
    fn claim_farm_boosted_rewards(
        &self,
        farm_address: ManagedAddress,
        user: ManagedAddress,
    ) -> EsdtTokenPayment {
        self.farm_proxy_obj(farm_address)
            .claim_boosted_rewards(user)
            .execute_on_dest_context()
    }

    #[proxy]
    fn farm_proxy_obj(
        &self,
        sc_address: ManagedAddress,
    ) -> claim_farm_boosted_rewards_proxy::Proxy<Self::Api>;

    #[view(getEnergyThreshold)]
    #[storage_mapper("energyThreshold")]
    fn energy_threshold(&self) -> SingleValueMapper<BigUint>;
}
