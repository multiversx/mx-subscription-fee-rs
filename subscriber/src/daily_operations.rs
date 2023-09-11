use core::borrow::Borrow;

use auto_farm::common::{address_to_id_mapper::AddressId, unique_payments::UniquePayments};
use multiversx_sc_modules::ongoing_operation::{CONTINUE_OP, STOP_OP};
use subscription_fee::subtract_payments::{MyVeryOwnScResult, ProxyTrait as _};

use crate::{base_functions::SubscriberContract, service::SubscriptionType};

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub type Epoch = u64;

pub const DAILY_EPOCHS: Epoch = 1;
pub const WEEKLY_EPOCHS: Epoch = 7;
pub const MONTHLY_EPOCHS: Epoch = 30;

pub const FIRST_INDEX: usize = 1;
pub const GAS_TO_SAVE_PROGRESS: u64 = 100_000;

#[derive(TypeAbi, TopEncode, TopDecode, Default)]
pub struct OperationProgress {
    pub service_index: usize,
    pub current_index: usize,
    pub additional_data_index: usize,
}

#[multiversx_sc::module]
pub trait DailyOperationsModule:
    crate::service::ServiceModule
    + crate::user_tokens::UserTokensModule
    + crate::common_storage::CommonStorageModule
    + energy_query::EnergyQueryModule
    + multiversx_sc_modules::ongoing_operation::OngoingOperationModule
{
    fn perform_service<SC: SubscriberContract<SubSc = Self>>(
        &self,
        service_index: usize,
        user_index: &mut usize,
        additional_data: ManagedVec<SC::AdditionalDataType>,
    ) -> OperationCompletionStatus {
        let mut users_mapper = self.subscribed_users(service_index);
        let mut total_users = users_mapper.len();
        let mut progress = self.load_operation::<OperationProgress>();
        if progress.current_index == 0 {
            progress.service_index = service_index;
            progress.current_index = *user_index;
            progress.additional_data_index = 0;
        } else {
            require!(
                progress.service_index == service_index,
                "Another operation is in progress"
            );
        }

        let fees_contract_address = self.fees_contract_address().get();
        let energy_threshold = self.energy_threshold().get();
        let service_info = self.service_info().get().get(service_index);
        let current_epoch = self.blockchain().get_block_epoch();
        let additional_data_len = additional_data.len();

        let run_result = self.run_while_it_has_gas(GAS_TO_SAVE_PROGRESS, || {
            if *user_index == additional_data_len || progress.current_index == total_users + 1 {
                return STOP_OP;
            }

            let user_data = additional_data.get(*user_index);
            *user_index += 1;

            let user_id = users_mapper.get_by_index(progress.current_index);
            let opt_user_address = self
                .user_id()
                .get_address_at_address(&fees_contract_address, user_id);
            let user_address = match opt_user_address {
                Some(address) => address,
                None => {
                    users_mapper.swap_remove(&user_id);
                    total_users -= 1;
                    return CONTINUE_OP;
                }
            };

            progress.current_index += 1;

            let subscription_type = self.subscription_type(user_id, service_index).get();
            let next_action_epoch =
                self.get_next_action_epoch(user_id, service_index, subscription_type);
            if next_action_epoch > current_epoch {
                return CONTINUE_OP;
            }

            let user_energy = self.get_energy_amount(&user_address);
            let is_premium_user = user_energy >= energy_threshold;
            let payment_amount = if is_premium_user {
                service_info.payment_type.amount_for_premium.clone()
            } else {
                service_info.payment_type.amount_for_normal.clone()
            };

            let subtract_result = self.subtract_payment(
                fees_contract_address.clone(),
                user_id,
                service_info.payment_type.opt_specific_token.clone(),
                payment_amount,
            );
            if subtract_result.is_err() {
                return CONTINUE_OP;
            }

            let action_results = SC::perform_action(
                self,
                user_address.clone(),
                user_id,
                &service_info,
                user_data.borrow(),
            );
            if action_results.is_err() {
                return CONTINUE_OP;
            }

            // return funds if it didn't work? - discuss

            let interpreted_results = unsafe { action_results.unwrap_unchecked() };
            if let Some(new_token) = interpreted_results.opt_new_token {
                self.save_new_token(user_id, new_token);
            }

            self.send_user_rewards(&user_address, interpreted_results.user_rewards);

            self.last_action_epoch(user_id, service_index)
                .set(current_epoch);

            CONTINUE_OP
        });

        if run_result == OperationCompletionStatus::InterruptedBeforeOutOfGas {
            self.save_progress(&progress);
        }

        run_result
    }

    fn subtract_payment(
        &self,
        fee_contract_address: ManagedAddress,
        user_id: AddressId,
        opt_specific_token: Option<EgldOrEsdtTokenIdentifier>,
        amount: BigUint,
    ) -> MyVeryOwnScResult<(), ()> {
        self.fee_contract_proxy_obj(fee_contract_address)
            .subtract_payment(user_id, opt_specific_token, amount)
            .execute_on_dest_context()
    }

    fn get_next_action_epoch(
        &self,
        user_id: AddressId,
        service_index: usize,
        subscription_type: SubscriptionType,
    ) -> Epoch {
        let last_action_epoch = self.last_action_epoch(user_id, service_index).get();
        match subscription_type {
            SubscriptionType::None => sc_panic!("Unexpected value"),
            SubscriptionType::Daily => last_action_epoch + DAILY_EPOCHS,
            SubscriptionType::Weekly => last_action_epoch + WEEKLY_EPOCHS,
            SubscriptionType::Monthly => last_action_epoch + MONTHLY_EPOCHS,
        }
    }

    fn save_new_token(&self, user_id: AddressId, new_token: EsdtTokenPayment) {
        let mapper = self.user_deposited_tokens(user_id);
        let mut tokens = mapper.get().into_payments();
        tokens.push(new_token);

        mapper.set(UniquePayments::new_from_unique_payments(tokens));
    }

    fn send_user_rewards(
        &self,
        user_address: &ManagedAddress,
        rewards: ManagedVec<EsdtTokenPayment>,
    ) {
        for rew in &rewards {
            self.send().direct_non_zero_esdt_payment(user_address, &rew);
        }
    }

    #[proxy]
    fn fee_contract_proxy_obj(
        &self,
        sc_address: ManagedAddress,
    ) -> subscription_fee::Proxy<Self::Api>;

    #[storage_mapper("lastActionEpoch")]
    fn last_action_epoch(
        &self,
        user_id: AddressId,
        service_index: usize,
    ) -> SingleValueMapper<Epoch>;

    #[storage_mapper("energyThreshold")]
    fn energy_threshold(&self) -> SingleValueMapper<BigUint>;
}
