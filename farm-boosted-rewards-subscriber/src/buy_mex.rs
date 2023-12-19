use auto_farm::common::unique_payments::UniquePayments;
use multiversx_sc_modules::ongoing_operation::{CONTINUE_OP, STOP_OP};
use subscription_fee::subtract_payments::Epoch;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub type Percentage = u32;
pub const TOTAL_PERCENTAGE: Percentage = 10_000;
pub const BUY_MEX_COST: u64 = 20_000_000;
pub const LOCK_GAS_PER_USER: u64 = 7_000_000;

#[derive(TypeAbi, TopEncode, TopDecode, PartialEq)]
pub enum SubscriptionUserType {
    Normal,
    Premium,
}

#[derive(TypeAbi, TopEncode, TopDecode)]
pub struct MexActionsPercentages {
    pub lock: Percentage,
    pub fees: Percentage,
    pub mex_burn: Percentage,
}

pub struct MexActionsValue<M: ManagedTypeApi> {
    pub lock: BigUint<M>,
    pub fees: BigUint<M>,
    pub mex_burn: BigUint<M>,
}

impl MexActionsPercentages {
    pub fn is_valid(&self) -> bool {
        self.lock + self.fees + self.mex_burn == TOTAL_PERCENTAGE
    }

    pub fn get_amounts_per_category<M: ManagedTypeApi>(
        &self,
        total: &BigUint<M>,
    ) -> MexActionsValue<M> {
        let lock_amount = total * self.lock / TOTAL_PERCENTAGE;
        let fees_amount = total * self.fees / TOTAL_PERCENTAGE;
        let mex_burn_amount = total - &lock_amount - &fees_amount;

        MexActionsValue {
            lock: lock_amount,
            fees: fees_amount,
            mex_burn: mex_burn_amount,
        }
    }
}

impl<M: ManagedTypeApi> MexActionsValue<M> {
    pub fn get_total_mex(&self) -> BigUint<M> {
        &self.lock + &self.mex_burn
    }
}

// dummy data is there so we don't accidentally deserialize another struct
#[derive(Default, TopEncode, TopDecode)]
pub struct MexOperationsProgress {
    pub service_index: usize,
    pub user_index: usize,
    pub dummy_data: u8,
}

#[multiversx_sc::module]
pub trait BuyMexModule:
    subscriber::service::ServiceModule
    + subscriber::common_storage::CommonStorageModule
    + multiversx_sc_modules::ongoing_operation::OngoingOperationModule
{
    #[only_owner]
    #[endpoint(addMexPair)]
    fn add_mex_pair(&self, token_id: TokenIdentifier, pair_address: ManagedAddress) {
        require!(token_id.is_valid_esdt_identifier(), "Invalid token ID");
        require!(
            self.blockchain().is_smart_contract(&pair_address),
            "Invalid pair address"
        );

        self.mex_pairs(&token_id).set(pair_address);
    }

    #[only_owner]
    #[endpoint(removeMexPair)]
    fn remove_mex_pair(&self, token_id: TokenIdentifier) {
        self.mex_pairs(&token_id).clear();
    }

    #[only_owner]
    #[endpoint(performMexOperations)]
    fn perform_mex_operations_endpoint(&self, service_index: usize) -> OperationCompletionStatus {
        let actions_percentage = if service_index == SubscriptionUserType::Normal as usize {
            self.normal_user_percentage().get()
        } else if service_index == SubscriptionUserType::Premium as usize {
            self.premium_user_percentage().get()
        } else {
            sc_panic!("Invalid service index")
        };

        let mut gas_per_user = BUY_MEX_COST;
        if actions_percentage.lock > 0 {
            gas_per_user += LOCK_GAS_PER_USER;
        }

        let mut progress = self.load_operation::<MexOperationsProgress>();
        if progress.user_index == 0 {
            progress.user_index = 1;
            progress.service_index = service_index;
        } else {
            require!(
                progress.service_index == service_index,
                "Another operation in progress"
            );
        }

        let own_address = self.blockchain().get_sc_address();
        let fees_contract_address = self.fees_contract_address().get();
        let service_id = self
            .service_id()
            .get_id_at_address_non_zero(&fees_contract_address, &own_address);

        let users_mapper = self.subscribed_users(service_id, service_index);
        let total_users = users_mapper.len_at_address(&fees_contract_address);

        let mut total_gas_cost = 0;
        let _ = self.run_while_it_has_gas(0, || {
            let remaining_gas = self.blockchain().get_gas_left();
            if total_gas_cost + gas_per_user > remaining_gas || progress.user_index > total_users {
                return STOP_OP;
            }

            total_gas_cost += gas_per_user;

            let user_id =
                users_mapper.get_by_index_at_address(&fees_contract_address, progress.user_index);
            progress.user_index += 1;

            let opt_user_address = self
                .user_id()
                .get_address_at_address(&fees_contract_address, user_id);
            if opt_user_address.is_none() {
                return CONTINUE_OP;
            }

            let fee_mapper = self.user_fees(service_index, user_id);
            if fee_mapper.is_empty() {
                return CONTINUE_OP;
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

            CONTINUE_OP
        });

        if progress.user_index <= total_users {
            self.save_progress(&progress);

            OperationCompletionStatus::InterruptedBeforeOutOfGas
        } else {
            OperationCompletionStatus::Completed
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
        let total_mex_to_buy = actions_value.get_total_mex();

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
        let mex_to_lock = &bought_mex.amount * actions_percentages.lock / TOTAL_PERCENTAGE;
        let mex_to_burn = bought_mex.amount - &mex_to_lock;

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
        let pair_mapper = self.mex_pairs(&token_id);
        require!(!pair_mapper.is_empty(), "No pair set for token");

        let mex_token_id = self.mex_token_id().get();
        let pair_address = pair_mapper.get();

        self.call_swap_to_mex(pair_address, mex_token_id, token_id, amount)
    }

    fn call_swap_to_mex(
        &self,
        pair_address: ManagedAddress,
        mex_token_id: TokenIdentifier,
        input_token_id: TokenIdentifier,
        amount: BigUint,
    ) -> EsdtTokenPayment {
        self.other_pair_proxy(pair_address)
            .swap_tokens_fixed_input(mex_token_id, BigUint::from(1u32))
            .with_esdt_transfer(EsdtTokenPayment::new(input_token_id, 0, amount))
            .execute_on_dest_context()
    }

    fn call_lock_tokens(
        &self,
        simple_lock_address: ManagedAddress,
        input_tokens: EsdtTokenPayment,
        lock_epochs: Epoch,
        destination: ManagedAddress,
    ) -> EsdtTokenPayment {
        self.simple_lock_proxy(simple_lock_address)
            .lock_tokens_endpoint(lock_epochs, destination)
            .with_esdt_transfer(input_tokens)
            .execute_on_dest_context()
    }

    #[proxy]
    fn other_pair_proxy(&self, sc_address: ManagedAddress) -> pair::Proxy<Self::Api>;

    #[proxy]
    fn simple_lock_proxy(&self, sc_address: ManagedAddress) -> energy_factory::Proxy<Self::Api>;

    #[storage_mapper("mexTokenId")]
    fn mex_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[storage_mapper("mexPairs")]
    fn mex_pairs(&self, token_id: &TokenIdentifier) -> SingleValueMapper<ManagedAddress>;

    #[storage_mapper("simpleLockAddress")]
    fn simple_lock_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[storage_mapper("lockPeriod")]
    fn lock_period(&self) -> SingleValueMapper<Epoch>;

    #[view(getNormalUserPercentage)]
    #[storage_mapper("normalUserPercentage")]
    fn normal_user_percentage(&self) -> SingleValueMapper<MexActionsPercentages>;

    #[view(getPremiumUserPercentage)]
    #[storage_mapper("premiumUserPercentage")]
    fn premium_user_percentage(&self) -> SingleValueMapper<MexActionsPercentages>;

    #[view(getTotalFees)]
    #[storage_mapper("totalFees")]
    fn total_fees(&self) -> SingleValueMapper<UniquePayments<Self::Api>>;
}
