multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use auto_farm::common::address_to_id_mapper::{AddressId, AddressToIdMapper};

#[multiversx_sc::module]
pub trait ClaimFarmBoostedRewardsModule:
    subscriber::common_storage::CommonStorageModule + energy_query::EnergyQueryModule
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
        require!(service_index <= 1, "Invalid service index");
        let energy_threshold = self.energy_threshold().get();
        let premium_service = service_index == 1;

        if premium_service {
            require!(energy_threshold > 0, "Invalid energy threshold");
        }

        let own_address = self.blockchain().get_sc_address();
        let fees_contract_address = self.fees_contract_address().get();
        let service_id = self
            .service_id()
            .get_id_at_address_non_zero(&fees_contract_address, &own_address);
        let subscribed_users = self.subscribed_users(service_id, service_index);

        for user_farms_pair in user_farms_pairs_to_claim {
            let (user_id, farms_ids) = user_farms_pair.into_tuple();
            if !subscribed_users.contains_at_address(&fees_contract_address, &user_id) {
                continue;
            }

            let opt_user = self
                .user_id()
                .get_address_at_address(&fees_contract_address, user_id);
            if opt_user.is_none() {
                continue;
            }
            let user = opt_user.unwrap();

            if premium_service {
                let user_energy = self.get_energy_amount(&user);

                if user_energy < energy_threshold {
                    continue;
                }
            }

            for farm_id in &farms_ids {
                let farm_address_opt = self.farm_id().get_address(farm_id);
                if farm_address_opt.is_some() {
                    let farm_address = farm_address_opt.unwrap();
                    self.claim_farm_boosted_rewards(farm_address, user.clone());
                }
            }
        }
    }

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
    ) -> farm_with_locked_rewards::Proxy<Self::Api>;

    #[view(getEnergyThreshold)]
    #[storage_mapper("energyThreshold")]
    fn energy_threshold(&self) -> SingleValueMapper<BigUint>;

    #[storage_mapper("farmId")]
    fn farm_id(&self) -> AddressToIdMapper<Self::Api>;
}
