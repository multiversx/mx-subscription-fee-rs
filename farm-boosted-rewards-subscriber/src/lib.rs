#![no_std]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use multiversx_sc_modules::only_admin;
use subscriber_config::MexActionsPercentages;
use subscription_fee::subtract_payments::Epoch;

pub mod claim_farm_boosted;
pub mod events;
pub mod service;
pub mod subscriber_config;

#[multiversx_sc::contract]
pub trait SubscriberContractMain:
    subscriber_config::SubscriberConfigModule
    + claim_farm_boosted::ClaimFarmBoostedRewardsModule
    + service::ServiceModule
    + common_subscriber::CommonSubscriberModule
    + energy_query::EnergyQueryModule
    + events::EventsModule
    + only_admin::OnlyAdminModule
{
    /// Percentages must add up to 10,000 each, where 10,000 = 100%
    /// Lock period is number of epochs the tokens should be locked for
    #[init]
    fn init(
        &self,
        fees_contract_address: ManagedAddress,
        energy_threshold: BigUint,
        mex_token_id: TokenIdentifier,
        wegld_token_id: TokenIdentifier,
        normal_user_percentages: MexActionsPercentages,
        premium_user_percentages: MexActionsPercentages,
        simple_lock_address: ManagedAddress,
        mex_pair_address: ManagedAddress,
        lock_period: Epoch,
        fees_claim_address: ManagedAddress,
    ) {
        require!(mex_token_id.is_valid_esdt_identifier(), "Invalid token ID");
        require!(
            wegld_token_id.is_valid_esdt_identifier(),
            "Invalid token ID"
        );
        require!(
            normal_user_percentages.is_valid() && premium_user_percentages.is_valid(),
            "Invalid percentages"
        );
        require!(
            self.blockchain().is_smart_contract(&simple_lock_address),
            "Invalid address"
        );
        require!(
            self.blockchain().is_smart_contract(&mex_pair_address),
            "Invalid pair address"
        );

        let first_token_id = self.first_token_id().get_from_address(&mex_pair_address);
        let second_token_id = self.second_token_id().get_from_address(&mex_pair_address);

        require!(
            first_token_id == wegld_token_id || first_token_id == mex_token_id,
            "Wrong pair address"
        );
        require!(
            second_token_id == wegld_token_id || second_token_id == mex_token_id,
            "Wrong pair address"
        );

        self.base_init(fees_contract_address);
        self.energy_threshold().set_if_empty(energy_threshold);
        self.mex_token_id().set_if_empty(mex_token_id);
        self.wegld_token_id().set_if_empty(wegld_token_id);
        self.normal_user_percentage()
            .set_if_empty(normal_user_percentages);
        self.premium_user_percentage()
            .set_if_empty(premium_user_percentages);
        self.simple_lock_address().set_if_empty(simple_lock_address);
        self.mex_pair().set_if_empty(mex_pair_address);
        self.lock_period().set_if_empty(lock_period);
        self.fees_claim_address().set_if_empty(fees_claim_address);
        self.add_admin(self.blockchain().get_caller());
    }

    #[upgrade]
    fn upgrade(&self) {}

    #[only_owner]
    #[endpoint(setLockPeriod)]
    fn set_lock_period(&self, lock_period: Epoch) {
        self.lock_period().set(lock_period);
    }

    #[only_owner]
    #[endpoint(setEnergyThreshold)]
    fn set_energy_threshold(&self, energy_threshold: BigUint) {
        self.energy_threshold().set(energy_threshold);
    }
}
