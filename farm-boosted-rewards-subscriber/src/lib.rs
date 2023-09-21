#![no_std]
#![feature(trait_alias)]

use core::marker::PhantomData;

use auto_farm::common::{address_to_id_mapper::AddressId, unique_payments::UniquePayments};
use buy_mex::MexActionsPercentages;
use subscriber::base_functions::{AllBaseTraits, InterpretedResult, SubscriberContract};
use subscription_fee::{service::ServiceInfo, subtract_payments::Epoch};

use crate::claim_farm_boosted::AdditionalFarmData;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub mod buy_mex;
pub mod claim_farm_boosted;

pub const LOCK_GAS_PER_USER: u64 = 7_000_000;

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

    #[endpoint(performAction)]
    fn perform_action_endpoint(
        &self,
        service_index: usize,
        users_to_claim: MultiValueEncoded<AddressId>,
    ) -> OperationCompletionStatus {
        require!(service_index <= 1, "invalid index");

        let total_users = users_to_claim.len();
        let mut args_vec = ManagedVec::new();
        for _ in 0..total_users {
            args_vec.push(AdditionalFarmData { dummy_data: 0 });
        }

        let current_epoch = self.blockchain().get_block_epoch();
        let mut user_index = self.get_user_index(service_index, current_epoch);
        let result = self.perform_service::<FarmClaimBoostedWrapper<Self>>(
            total_users as u64 * LOCK_GAS_PER_USER,
            service_index,
            &mut user_index,
            args_vec,
        );

        // do stuff with result

        self.user_index().set(user_index);
        self.last_global_action_epoch(service_index)
            .set(current_epoch);

        result.status
    }
}

pub struct FarmClaimBoostedWrapper<
    T: AllBaseTraits
        + buy_mex::BuyMexModule
        + claim_farm_boosted::ClaimFarmBoostedRewardsModule
        + energy_query::EnergyQueryModule,
> {
    _phantom: PhantomData<T>,
}

impl<T> SubscriberContract for FarmClaimBoostedWrapper<T>
where
    T: AllBaseTraits
        + buy_mex::BuyMexModule
        + claim_farm_boosted::ClaimFarmBoostedRewardsModule
        + energy_query::EnergyQueryModule,
{
    type SubSc = T;
    type AdditionalDataType = AdditionalFarmData;

    fn perform_action(
        sc: &Self::SubSc,
        user_address: ManagedAddress<<Self::SubSc as ContractBase>::Api>,
        fee: EgldOrEsdtTokenPayment<<Self::SubSc as ContractBase>::Api>,
        service_index: usize,
        service_info: &ServiceInfo<<Self::SubSc as ContractBase>::Api>,
        _additional_data: &<Self as SubscriberContract>::AdditionalDataType,
    ) -> Result<InterpretedResult<<Self::SubSc as ContractBase>::Api>, ()> {
        if service_index == 1 {
            let user_energy = sc.get_energy_amount(&user_address);
            let energy_threshold = sc.energy_threshold().get();
            if user_energy < energy_threshold {
                return Result::Err(());
            }
        }

        let actions_percentage = if service_index == 0 {
            sc.normal_user_percentage().get()
        } else {
            sc.premium_user_percentage().get()
        };

        let token_id = if fee.token_identifier.is_egld() {
            // wrap egld
            TokenIdentifier::from("PLACEHOLDER")
        } else {
            fee.token_identifier.unwrap_esdt()
        };

        sc.perform_mex_operations(
            user_address.clone(),
            token_id,
            fee.amount,
            &actions_percentage,
        );

        let _ = sc.claim_farm_boosted_rewards(service_info.sc_address.clone(), user_address);

        // farm already sent rewards to user
        Result::Ok(InterpretedResult {
            user_rewards: ManagedVec::new(),
        })
    }
}
