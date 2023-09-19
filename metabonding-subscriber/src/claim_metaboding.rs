multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use metabonding::{rewards::Week, validation::Signature};
use multiversx_sc_modules::transfer_role_proxy::PaymentsVec;

mod claim_metaboding_mod {
    use metabonding::claim::ClaimArgPair;
    use multiversx_sc_modules::transfer_role_proxy::PaymentsVec;

    multiversx_sc::imports!();

    #[multiversx_sc::proxy]
    pub trait ClaimMetabodingRewardsProxy {
        #[endpoint(claimRewards)]
        fn claim_rewards(
            &self,
            original_caller: ManagedAddress,
            raw_claim_args: MultiValueEncoded<ClaimArgPair<Self::Api>>,
        ) -> PaymentsVec<Self::Api>;
    }
}

#[derive(ManagedVecItem, Clone)]
pub struct AdditionalMetabodingData<M: ManagedTypeApi> {
    pub week: Week,
    pub user_delegation_amount: BigUint<M>,
    pub user_lkmex_staked_amount: BigUint<M>,
    pub signature: Signature<M>,
}

#[multiversx_sc::module]
pub trait ClaimMetabondingModule {
    fn claim_metaboding_rewards(
        &self,
        metaboding_address: ManagedAddress,
        user_address: ManagedAddress,
        claim_args: &AdditionalMetabodingData<Self::Api>,
    ) -> PaymentsVec<Self::Api> {
        let mut raw_claim_args = MultiValueEncoded::new();
        let raw_arg = (
            claim_args.week,
            claim_args.user_delegation_amount.clone(),
            claim_args.user_lkmex_staked_amount.clone(),
            claim_args.signature.clone(),
        );
        raw_claim_args.push(raw_arg.into());

        self.metaboding_proxy_obj(metaboding_address)
            .claim_rewards(user_address, raw_claim_args)
            .execute_on_dest_context()
    }

    #[proxy]
    fn metaboding_proxy_obj(
        &self,
        sc_address: ManagedAddress,
    ) -> claim_metaboding_mod::Proxy<Self::Api>;

    #[view(getEnergyThreshold)]
    #[storage_mapper("energyThreshold")]
    fn energy_threshold(&self) -> SingleValueMapper<BigUint>;
}
