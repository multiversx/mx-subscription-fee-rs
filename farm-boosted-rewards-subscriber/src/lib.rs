#![no_std]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use auto_farm::common::unique_payments::UniquePayments;
use subscriber_config::MexActionsPercentages;
use subscription_fee::subtract_payments::Epoch;

pub mod claim_farm_boosted;
pub mod service;
pub mod subscriber_config;

#[multiversx_sc::contract]
pub trait SubscriberContractMain:
    subscriber_config::SubscriberConfigModule
    + claim_farm_boosted::ClaimFarmBoostedRewardsModule
    + service::ServiceModule
    + common_subscriber::CommonSubscriberModule
    + energy_query::EnergyQueryModule
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

        self.base_init(fees_contract_address);
        self.energy_threshold().set(energy_threshold);
        self.mex_token_id().set(mex_token_id);
        self.wegld_token_id().set(wegld_token_id);
        self.normal_user_percentage().set(normal_user_percentages);
        self.premium_user_percentage().set(premium_user_percentages);
        self.simple_lock_address().set(simple_lock_address);
        self.mex_pair().set(mex_pair_address);
        self.lock_period().set(lock_period);

        self.total_fees().set(UniquePayments::new());
    }

    #[endpoint]
    fn upgrade(&self, energy_threshold: BigUint, lock_period: Epoch) {
        self.energy_threshold().set(energy_threshold);
        self.lock_period().set(lock_period);
    }
}
