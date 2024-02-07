multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use multiversx_sc_modules::only_admin;

use crate::events;
use crate::events::ClaimRewardsOperation;
use crate::service;
use crate::subscriber_config;
use crate::subscriber_config::SubscriptionUserType;

#[multiversx_sc::module]
pub trait ClaimFarmBoostedRewardsModule:
    subscriber_config::SubscriberConfigModule
    + service::ServiceModule
    + common_subscriber::CommonSubscriberModule
    + energy_query::EnergyQueryModule
    + events::EventsModule
    + only_admin::OnlyAdminModule
{
    #[only_owner]
    #[endpoint(addFarm)]
    fn add_farm(&self, farm_address: ManagedAddress) -> AddressId {
        require!(
            self.blockchain().is_smart_contract(&farm_address),
            "Invalid farm address"
        );

        self.farm_id().insert_new(&farm_address)
    }

    #[only_owner]
    #[endpoint(removeFarm)]
    fn remove_farm(&self, farm_address: ManagedAddress) -> AddressId {
        self.farm_id().remove_by_address(&farm_address)
    }

    #[endpoint(performClaimRewardsOperations)]
    fn perform_claim_rewards_operations_endpoint(
        &self,
        service_index: usize,
        user_farms_pairs_to_claim: MultiValueEncoded<MultiValue2<AddressId, ManagedVec<AddressId>>>,
    ) {
        require!(
            service_index == SubscriptionUserType::Normal as usize
                || service_index == SubscriptionUserType::Premium as usize,
            "Invalid service index"
        );

        let fees_contract_address = self.fees_contract_address().get();

        let mut claim_reward_operations = ManagedVec::new();
        for user_farms_pair in user_farms_pairs_to_claim {
            let (user_id, farms_ids) = user_farms_pair.into_tuple();

            let opt_user = self
                .user_id()
                .get_address_at_address(&fees_contract_address, user_id);
            if opt_user.is_none() {
                continue;
            }
            let user = opt_user.unwrap();

            let mut processed_farms = ManagedVec::new();
            for farm_id in &farms_ids {
                let farm_address_opt = self.farm_id().get_address(farm_id);
                if farm_address_opt.is_some() {
                    let farm_address = farm_address_opt.unwrap();
                    if !self
                        .get_user_allow_claim_boosted_rewards(farm_address.clone(), user.clone())
                    {
                        continue;
                    }
                    self.claim_boosted_rewards(farm_address, user.clone());
                    processed_farms.push(farm_id);
                }
            }

            if !processed_farms.is_empty() {
                claim_reward_operations.push(ClaimRewardsOperation::new(user, processed_farms));
            }
        }

        self.emit_claim_rewards_event(claim_reward_operations);
    }
}
