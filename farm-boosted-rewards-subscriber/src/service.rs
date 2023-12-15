multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use crate::subscriber_config::{self, MexActionsPercentages, SubscriptionUserType};

#[multiversx_sc::module]
pub trait ServiceModule:
    subscriber_config::SubscriberConfigModule
    + common_subscriber::CommonSubscriberModule
    + energy_query::EnergyQueryModule
{
    #[only_owner]
    #[endpoint(subtractPayment)]
    fn subtract_payment_endpoint(
        &self,
        service_index: usize,
        user_ids: MultiValueEncoded<AddressId>,
    ) {
        require!(service_index <= 1, "Invalid service index");
        let premium_service = service_index == 1;
        let energy_threshold = self.energy_threshold().get();

        let fees_contract_address = self.fees_contract_address().get();

        for user_id in user_ids {
            if premium_service {
                let opt_user_address = self
                    .user_id()
                    .get_address_at_address(&fees_contract_address, user_id);
                if opt_user_address.is_none() {
                    continue;
                }
                let user = opt_user_address.unwrap();
                let user_energy = self.get_energy_amount(&user);

                if user_energy < energy_threshold {
                    continue;
                }
            }

            self.subtract_user_payment(fees_contract_address.clone(), service_index, user_id);
        }
    }

    #[only_owner]
    #[endpoint(claimFees)]
    fn claim_fees(&self) -> ManagedVec<EsdtTokenPayment> {
        let caller = self.blockchain().get_caller();
        let total_fees = self.total_fees().take().into_payments();
        self.send().direct_multi(&caller, &total_fees);

        total_fees
    }

    #[only_owner]
    #[endpoint(performMexOperations)]
    fn perform_mex_operations_endpoint(
        &self,
        service_index: usize,
        user_ids: MultiValueEncoded<AddressId>,
    ) {
        let actions_percentage = if service_index == SubscriptionUserType::Normal as usize {
            self.normal_user_percentage().get()
        } else if service_index == SubscriptionUserType::Premium as usize {
            self.premium_user_percentage().get()
        } else {
            sc_panic!("Invalid service index")
        };

        let fees_contract_address = self.fees_contract_address().get();

        for user_id in user_ids {
            let opt_user_address = self
                .user_id()
                .get_address_at_address(&fees_contract_address, user_id);
            if opt_user_address.is_none() {
                continue;
            }

            let fee_mapper = self.user_fees(service_index, user_id);
            if fee_mapper.is_empty() {
                continue;
            }

            let fee = fee_mapper.take();
            let token_id = fee.fees.token_identifier;

            let user_address = unsafe { opt_user_address.unwrap_unchecked() };
            self.perform_mex_operations(
                user_address,
                token_id,
                fee.fees.amount,
                &actions_percentage,
            );
        }
    }

    fn perform_mex_operations(
        &self,
        user_address: ManagedAddress,
        token_id: TokenIdentifier,
        total_tokens: BigUint,
        actions_percentages: &MexActionsPercentages,
    ) {
        let actions_value = actions_percentages.get_amounts_per_category(&total_tokens);
        let total_mex_to_buy = actions_value.get_total_mex_to_buy();

        if actions_value.fees > 0 {
            self.total_fees().update(|fees| {
                fees.add_payment(EsdtTokenPayment::new(
                    token_id.clone(),
                    0,
                    actions_value.fees.clone(),
                ))
            });
        }

        let bought_mex = self.buy_mex(token_id, total_mex_to_buy);
        let mex_to_lock = &bought_mex.amount * actions_percentages.lock
            / (actions_percentages.lock + actions_percentages.burn);
        let mex_to_burn = &bought_mex.amount - &mex_to_lock;

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

    fn buy_mex(&self, token_id: TokenIdentifier, amount: BigUint) -> EsdtTokenPayment {
        let pair_mapper = self.mex_pair();
        require!(!pair_mapper.is_empty(), "The MEX pair is not set");

        let mex_token_id = self.mex_token_id().get();
        let pair_address = pair_mapper.get();

        self.call_swap_to_mex(pair_address, mex_token_id, token_id, amount)
    }
}
