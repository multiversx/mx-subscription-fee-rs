#![no_std]
#![feature(trait_alias)]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use auto_farm::common::unique_payments::UniquePayments;
use buy_mex::MexActionsPercentages;
use subscription_fee::subtract_payments::Epoch;

pub mod buy_mex;
pub mod claim_farm_boosted;

#[multiversx_sc::contract]
pub trait SubscriberContractMain:
    claim_farm_boosted::ClaimFarmBoostedRewardsModule
    + buy_mex::BuyMexModule
    + subscriber::base_init::BaseInitModule
    + subscriber::service::ServiceModule
    + subscriber::daily_operations::DailyOperationsModule
    + subscriber::common_storage::CommonStorageModule
    + energy_query::EnergyQueryModule
    + multiversx_sc_modules::ongoing_operation::OngoingOperationModule
{
    /// Percentages must add up to 10,000 each, where 10,000 = 100%
    /// Lock period is number of epochs the tokens should be locked for
    #[init]
    fn init(
        &self,
        fees_contract_address: ManagedAddress,
        energy_threshold: BigUint,
        mex_token_id: TokenIdentifier,
        normal_user_percentages: MexActionsPercentages,
        premium_user_percentages: MexActionsPercentages,
        simple_lock_address: ManagedAddress,
        lock_period: Epoch,
    ) {
        require!(mex_token_id.is_valid_esdt_identifier(), "Invalid token ID");
        require!(
            normal_user_percentages.is_valid() && premium_user_percentages.is_valid(),
            "Invalid percentages"
        );
        require!(
            self.blockchain().is_smart_contract(&simple_lock_address),
            "Invalid address"
        );

        self.base_init(fees_contract_address);
        self.energy_threshold().set(energy_threshold);
        self.mex_token_id().set(mex_token_id);
        self.normal_user_percentage().set(normal_user_percentages);
        self.premium_user_percentage().set(premium_user_percentages);
        self.simple_lock_address().set(simple_lock_address);
        self.lock_period().set(lock_period);

        self.total_fees().set(UniquePayments::new());
    }
}
