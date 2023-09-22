use core::borrow::Borrow;

use auto_farm::common::address_to_id_mapper::AddressId;
use multiversx_sc::api::StorageMapperApi;
use multiversx_sc_modules::ongoing_operation::{LoopOp, CONTINUE_OP, STOP_OP};
use subscription_fee::{service::ServiceInfo, subtract_payments::Epoch};

use crate::base_functions::SubscriberContract;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub const FIRST_INDEX: usize = 1;

#[derive(TypeAbi, TopEncode, TopDecode, Default)]
pub struct ClaimRewardsOperationProgress {
    pub service_index: usize,
    pub user_index: usize,
    pub additional_data_index: usize,
}

pub struct OperationData<
    M: ManagedTypeApi + StorageMapperApi,
    AdditionalDataType: ManagedVecItem + Clone,
> {
    pub additional_data: ManagedVec<M, AdditionalDataType>,
    pub additional_data_len: usize,
    pub user_index: usize,
    pub total_users: usize,
    pub users_mapper: UnorderedSetMapper<M, AddressId>,
    pub service_info: ServiceInfo<M>,
    pub service_index: usize,
    pub fees_contract_address: ManagedAddress<M>,
}

#[multiversx_sc::module]
pub trait DailyOperationsModule:
    crate::service::ServiceModule
    + crate::common_storage::CommonStorageModule
    + multiversx_sc_modules::ongoing_operation::OngoingOperationModule
{
    fn perform_service<SC: SubscriberContract<SubSc = Self>>(
        &self,
        gas_to_save_progress: u64,
        service_index: usize,
        user_index: &mut usize,
        additional_data: ManagedVec<SC::AdditionalDataType>,
    ) -> OperationCompletionStatus {
        let own_address = self.blockchain().get_sc_address();
        let fees_contract_address = self.fees_contract_address().get();
        let service_id = self
            .service_id()
            .get_id_at_address_non_zero(&fees_contract_address, &own_address);

        let mut progress = self.load_operation::<ClaimRewardsOperationProgress>();
        if progress.user_index == 0 {
            progress.service_index = service_index;
            progress.user_index = *user_index;
            progress.additional_data_index = 0;
        } else {
            require!(
                progress.service_index == service_index,
                "Another operation is in progress"
            );
        }

        let users_mapper = self.subscribed_users(service_id, service_index);
        let total_users = users_mapper.len_at_address(&fees_contract_address);

        let mut all_data = OperationData::<Self::Api, SC::AdditionalDataType> {
            additional_data_len: additional_data.len(),
            additional_data,
            user_index: *user_index,
            total_users,
            users_mapper,
            service_info: self
                .service_info(service_id)
                .get_from_address(&fees_contract_address)
                .get(service_index),
            service_index,
            fees_contract_address,
        };

        let run_result = self.run_while_it_has_gas(gas_to_save_progress, || {
            self.perform_one_operation::<SC>(&mut progress, &mut all_data)
        });

        if run_result == OperationCompletionStatus::InterruptedBeforeOutOfGas {
            self.save_progress(&progress);
        }

        *user_index = progress.user_index;

        run_result
    }

    fn perform_one_operation<SC: SubscriberContract<SubSc = Self>>(
        &self,
        progress: &mut ClaimRewardsOperationProgress,
        all_data: &mut OperationData<Self::Api, SC::AdditionalDataType>,
    ) -> LoopOp {
        if progress.additional_data_index >= all_data.additional_data_len
            || progress.user_index > all_data.total_users
        {
            return STOP_OP;
        }

        let user_data = all_data.additional_data.get(progress.additional_data_index);
        progress.additional_data_index += 1;

        let user_id = all_data
            .users_mapper
            .get_by_index_at_address(&all_data.fees_contract_address, progress.user_index);
        let opt_user_address = self
            .user_id()
            .get_address_at_address(&all_data.fees_contract_address, user_id);
        let user_address = match opt_user_address {
            Some(address) => address,
            None => {
                return CONTINUE_OP;
            }
        };

        progress.user_index += 1;

        let action_results = SC::perform_action(
            self,
            user_address.clone(),
            all_data.service_index,
            &all_data.service_info,
            user_data.borrow(),
        );
        if let Result::Ok(interpreted_results) = action_results {
            self.send_user_rewards(&user_address, interpreted_results.user_rewards);
        }

        CONTINUE_OP
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

    fn get_user_index(&self, service_index: usize, current_epoch: Epoch) -> usize {
        let last_action_epoch = self.last_global_action_epoch(service_index).get();
        if last_action_epoch == current_epoch {
            self.user_index().get()
        } else {
            1
        }
    }

    #[storage_mapper("userIndex")]
    fn user_index(&self) -> SingleValueMapper<usize>;

    #[storage_mapper("lastGloblalActionEpoch")]
    fn last_global_action_epoch(&self, service_index: usize) -> SingleValueMapper<Epoch>;
}
