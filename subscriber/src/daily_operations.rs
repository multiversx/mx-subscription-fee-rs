use core::borrow::Borrow;

use auto_farm::common::address_to_id_mapper::AddressId;
use multiversx_sc::api::StorageMapperApi;
use multiversx_sc_modules::ongoing_operation::{LoopOp, CONTINUE_OP, STOP_OP};
use subscription_fee::{
    service::ServiceInfo,
    subtract_payments::{MyVeryOwnScResult, ProxyTrait as _},
};

use crate::base_functions::SubscriberContract;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub const FIRST_INDEX: usize = 1;
pub const GAS_TO_SAVE_PROGRESS: u64 = 100_000;

#[derive(TypeAbi, TopEncode, TopDecode, Default)]
pub struct OperationProgress {
    pub service_index: usize,
    pub current_index: usize,
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
        service_index: usize,
        user_index: &mut usize,
        additional_data: ManagedVec<SC::AdditionalDataType>,
    ) -> OperationCompletionStatus {
        let own_address = self.blockchain().get_sc_address();
        let fees_contract_address = self.fees_contract_address().get();
        let service_id = self
            .service_id()
            .get_id_at_address_non_zero(&fees_contract_address, &own_address);

        let users_mapper = self.subscribed_users(service_id, service_index);
        let total_users = users_mapper.len_at_address(&fees_contract_address);
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

        let mut output_egld = BigUint::zero();
        let mut output_esdt = ManagedVec::new();

        let run_result = self.run_while_it_has_gas(GAS_TO_SAVE_PROGRESS, || {
            self.perform_one_operation::<SC>(
                &mut progress,
                &mut all_data,
                &mut output_egld,
                &mut output_esdt,
            )
        });

        if run_result == OperationCompletionStatus::InterruptedBeforeOutOfGas {
            self.save_progress(&progress);
        }

        *user_index = progress.additional_data_index;

        let caller = self.blockchain().get_caller();
        if output_egld > 0 {
            self.send().direct_egld(&caller, &output_egld);
        }
        if !output_esdt.is_empty() {
            self.send().direct_multi(&caller, &output_esdt);
        }

        run_result
    }

    fn perform_one_operation<SC: SubscriberContract<SubSc = Self>>(
        &self,
        progress: &mut OperationProgress,
        all_data: &mut OperationData<Self::Api, SC::AdditionalDataType>,
        output_egld: &mut BigUint,
        output_esdt: &mut ManagedVec<EsdtTokenPayment>,
    ) -> LoopOp {
        if progress.additional_data_index >= all_data.additional_data_len
            || progress.current_index > all_data.total_users
        {
            return STOP_OP;
        }

        let user_data = all_data.additional_data.get(all_data.user_index);
        progress.additional_data_index += 1;

        let user_id = all_data
            .users_mapper
            .get_by_index_at_address(&all_data.fees_contract_address, progress.current_index);
        let opt_user_address = self
            .user_id()
            .get_address_at_address(&all_data.fees_contract_address, user_id);
        let user_address = match opt_user_address {
            Some(address) => address,
            None => {
                return CONTINUE_OP;
            }
        };

        progress.current_index += 1;

        let subtract_result = self.call_subtract_payment(
            all_data.fees_contract_address.clone(),
            all_data.service_index,
            user_id,
        );
        if subtract_result.is_err() {
            return CONTINUE_OP;
        }

        let fee = unsafe { subtract_result.unwrap_unchecked() };
        if fee.token_identifier.is_egld() {
            *output_egld += fee.amount;
        } else {
            let payment = EsdtTokenPayment::new(fee.token_identifier.unwrap_esdt(), 0, fee.amount);
            output_esdt.push(payment);
        }

        let action_results = SC::perform_action(
            self,
            user_address.clone(),
            user_id,
            all_data.service_index,
            &all_data.service_info,
            user_data.borrow(),
        );
        if action_results.is_err() {
            return CONTINUE_OP;
        }

        let interpreted_results = unsafe { action_results.unwrap_unchecked() };
        self.send_user_rewards(&user_address, interpreted_results.user_rewards);

        CONTINUE_OP
    }

    fn call_subtract_payment(
        &self,
        fee_contract_address: ManagedAddress,
        service_index: usize,
        user_id: AddressId,
    ) -> MyVeryOwnScResult<EgldOrEsdtTokenPayment, ()> {
        self.fee_contract_proxy_obj(fee_contract_address)
            .subtract_payment(service_index, user_id)
            .execute_on_dest_context()
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
}
