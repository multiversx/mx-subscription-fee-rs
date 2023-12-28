multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use crate::{
    events,
    subscriber_config::{self, MexActionsPercentages, SubscriptionUserType},
};

#[derive(ManagedVecItem, TypeAbi, TopEncode, TopDecode, PartialEq)]
pub struct MexOperationItem<M: ManagedTypeApi> {
    pub user_address: ManagedAddress<M>,
    pub amount: BigUint<M>,
}

impl<M: ManagedTypeApi> MexOperationItem<M> {
    pub fn new(user_address: ManagedAddress<M>, amount: BigUint<M>) -> Self {
        MexOperationItem {
            user_address,
            amount,
        }
    }
}

#[multiversx_sc::module]
pub trait ServiceModule:
    subscriber_config::SubscriberConfigModule
    + common_subscriber::CommonSubscriberModule
    + energy_query::EnergyQueryModule
    + events::EventsModule
{
    #[only_owner]
    #[endpoint(subtractPayment)]
    fn subtract_payment_endpoint(
        &self,
        service_index: usize,
        user_ids: MultiValueEncoded<AddressId>,
    ) {
        require!(
            service_index == SubscriptionUserType::Normal as usize
                || service_index == SubscriptionUserType::Premium as usize,
            "Invalid service index"
        );
        let is_premium_service = service_index == SubscriptionUserType::Premium as usize;

        let energy_threshold = self.energy_threshold().get();

        let fees_contract_address = self.fees_contract_address().get();

        let mut processed_user_ids = ManagedVec::new();
        for user_id in user_ids {
            if is_premium_service {
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
            processed_user_ids.push(user_id);
        }

        self.emit_subtract_payment_event(service_index, processed_user_ids);
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
        total_min_amount_out: BigUint,
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
        let wegld_token_id = self.wegld_token_id().get();

        let mut total_fees = BigUint::zero();
        let mut mex_operations_list: ManagedVec<MexOperationItem<Self::Api>> = ManagedVec::new();
        let mut processed_user_ids = ManagedVec::new();
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
            require!(token_id == wegld_token_id, "Invalid fee token id");
            let user_address = unsafe { opt_user_address.unwrap_unchecked() };

            total_fees += &fee.fees.amount;
            let mex_operation = MexOperationItem::new(user_address, fee.fees.amount);
            mex_operations_list.push(mex_operation);
            processed_user_ids.push(user_id);
        }

        let total_tokens_to_lock = self.perform_mex_operation(
            wegld_token_id,
            total_fees.clone(),
            &actions_percentage,
            total_min_amount_out,
        );

        if total_tokens_to_lock.amount == 0 {
            return;
        }

        let simple_lock_address = self.simple_lock_address().get();
        let lock_period = self.lock_period().get();

        // Call lock for each user to properly update their energy
        for mex_operation in mex_operations_list.into_iter() {
            let user_amount = &total_tokens_to_lock.amount * &mex_operation.amount / &total_fees;
            if user_amount > 0 {
                self.call_lock_tokens(
                    simple_lock_address.clone(),
                    EsdtTokenPayment::new(
                        total_tokens_to_lock.token_identifier.clone(),
                        0,
                        user_amount,
                    ),
                    lock_period,
                    mex_operation.user_address,
                );
            }
        }

        self.emit_mex_operation_event(service_index, processed_user_ids);
    }

    fn perform_mex_operation(
        &self,
        token_id: TokenIdentifier,
        total_tokens: BigUint,
        actions_percentages: &MexActionsPercentages,
        total_min_amount_out: BigUint,
    ) -> EsdtTokenPayment {
        let actions_value = actions_percentages.get_amounts_per_category(&total_tokens);
        let total_sell_amount = actions_value.get_sell_amount();

        if actions_value.fees > 0 {
            self.total_fees().update(|fees| {
                fees.add_payment(EsdtTokenPayment::new(
                    token_id.clone(),
                    0,
                    actions_value.fees.clone(),
                ))
            });
        }

        let bought_mex = self.buy_mex(token_id, total_sell_amount, total_min_amount_out);
        let mex_to_lock = &bought_mex.amount * actions_percentages.lock
            / (actions_percentages.lock + actions_percentages.burn);
        let mex_to_burn = &bought_mex.amount - &mex_to_lock;

        if mex_to_burn > 0 {
            self.send()
                .esdt_local_burn(&bought_mex.token_identifier, 0, &mex_to_burn);
        }

        EsdtTokenPayment::new(bought_mex.token_identifier, 0, mex_to_lock)
    }

    fn buy_mex(
        &self,
        token_id: TokenIdentifier,
        amount: BigUint,
        min_amount_out: BigUint,
    ) -> EsdtTokenPayment {
        let pair_mapper = self.mex_pair();
        require!(!pair_mapper.is_empty(), "The MEX pair is not set");

        let mex_token_id = self.mex_token_id().get();
        let pair_address = pair_mapper.get();

        self.call_swap_to_mex(pair_address, mex_token_id, token_id, amount, min_amount_out)
    }
}
