use auto_farm::common::address_to_id_mapper::AddressId;
use multiversx_sc::api::StorageMapperApi;
use multiversx_sc_modules::ongoing_operation::{LoopOp, CONTINUE_OP, STOP_OP};
use subscription_fee::{
    service::ProxyTrait as _,
    subtract_payments::{Epoch, MyVeryOwnScResult, ProxyTrait as _, MONTHLY_EPOCHS},
};

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

// Careful to not have this be the exact same size as ClaimRewardsOperationProgress struct!!!
#[derive(TypeAbi, TopEncode, TopDecode, Default)]
pub struct SubtractPaymentOperation {
    pub service_index: usize,
    pub user_index: usize,
}

#[derive(TypeAbi, TopEncode, TopDecode)]
pub struct UserFees<M: ManagedTypeApi> {
    pub fees: EgldOrEsdtTokenPayment<M>,
    pub epoch: Epoch,
}

pub struct PaymentOperationData<M: ManagedTypeApi + StorageMapperApi> {
    pub total_users: usize,
    pub service_index: usize,
    pub current_epoch: Epoch,
    pub fees_contract_address: ManagedAddress<M>,
    pub users_mapper: UnorderedSetMapper<M, AddressId>,
}

pub const GAS_TO_SAVE_PAYMENT_PROGRESS: u64 = 200_000;

#[multiversx_sc::module]
pub trait ServiceModule:
    crate::common_storage::CommonStorageModule
    + multiversx_sc_modules::ongoing_operation::OngoingOperationModule
{
    /// Arguments are pairs of sc_address, opt_payment_token and payment_amount
    #[only_owner]
    #[endpoint(registerService)]
    fn register_service(
        &self,
        args: MultiValueEncoded<
            MultiValue3<ManagedAddress, Option<EgldOrEsdtTokenIdentifier>, BigUint>,
        >,
    ) {
        let fees_contract_address = self.fees_contract_address().get();
        let _: () = self
            .register_service_proxy_obj(fees_contract_address)
            .register_service(args)
            .execute_on_dest_context();
    }

    #[only_owner]
    #[endpoint(unregisterService)]
    fn unregister_service(&self) {
        let fees_contract_address = self.fees_contract_address().get();
        let _: () = self
            .register_service_proxy_obj(fees_contract_address)
            .unregister_service()
            .execute_on_dest_context();
    }

    #[only_owner]
    #[endpoint(subtractPayment)]
    fn subtract_payment_endpoint(&self, service_index: usize) -> OperationCompletionStatus {
        let own_address = self.blockchain().get_sc_address();
        let fees_contract_address = self.fees_contract_address().get();
        let service_id = self
            .service_id()
            .get_id_at_address_non_zero(&fees_contract_address, &own_address);
        let current_epoch = self.blockchain().get_block_epoch();
        let next_sub_epoch = self.next_subtract_epoch(service_index).get();
        require!(
            current_epoch >= next_sub_epoch,
            "Cannot subtract payment yet!"
        );

        let mut user_index =
            self.get_subtract_user_index(&fees_contract_address, service_id, service_index);
        let mut progress = self.load_operation::<SubtractPaymentOperation>();
        if progress.user_index == 0 {
            progress.service_index = service_index;
            progress.user_index = user_index;
        } else {
            require!(
                progress.service_index == service_index,
                "Another operation is in progress"
            );
        }

        let users_mapper = self.subscribed_users(service_id, service_index);
        let total_users = users_mapper.len_at_address(&fees_contract_address);
        let all_data = PaymentOperationData {
            current_epoch,
            fees_contract_address,
            service_index,
            total_users,
            users_mapper,
        };

        let run_result = self.run_while_it_has_gas(GAS_TO_SAVE_PAYMENT_PROGRESS, || {
            self.perform_one_sub_operation(&mut progress, &all_data)
        });

        if run_result == OperationCompletionStatus::InterruptedBeforeOutOfGas {
            self.save_progress(&progress);
        }

        user_index = progress.user_index;

        self.subtract_user_index().set(user_index);

        run_result
    }

    fn perform_one_sub_operation(
        &self,
        progress: &mut SubtractPaymentOperation,
        all_data: &PaymentOperationData<Self::Api>,
    ) -> LoopOp {
        if progress.user_index > all_data.total_users {
            self.next_subtract_epoch(all_data.service_index)
                .set(all_data.current_epoch + MONTHLY_EPOCHS);

            return STOP_OP;
        }

        let user_id = all_data
            .users_mapper
            .get_by_index_at_address(&all_data.fees_contract_address, progress.user_index);
        let opt_user_address = self
            .user_id()
            .get_address_at_address(&all_data.fees_contract_address, user_id);
        if opt_user_address.is_none() {
            return CONTINUE_OP;
        }

        progress.user_index += 1;

        let fees_mapper = self.user_fees(all_data.service_index, user_id);
        require!(fees_mapper.is_empty(), "Last fees not processed yet");

        let subtract_result = self.call_subtract_payment(
            all_data.fees_contract_address.clone(),
            all_data.service_index,
            user_id,
        );
        if let MyVeryOwnScResult::Ok(fees) = subtract_result {
            let user_fees = UserFees {
                fees,
                epoch: all_data.current_epoch,
            };

            fees_mapper.set(user_fees);
        }

        CONTINUE_OP
    }

    fn get_subtract_user_index(
        &self,
        fees_contract_address: &ManagedAddress,
        service_id: AddressId,
        service_index: usize,
    ) -> usize {
        let last_user_index = self
            .subscribed_users(service_id, service_index)
            .len_at_address(fees_contract_address);
        let stored_user_index = self.subtract_user_index().get();

        if stored_user_index != 0 && stored_user_index < last_user_index {
            stored_user_index
        } else {
            1
        }
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

    #[proxy]
    fn register_service_proxy_obj(
        &self,
        sc_address: ManagedAddress,
    ) -> subscription_fee::Proxy<Self::Api>;

    #[proxy]
    fn fee_contract_proxy_obj(
        &self,
        sc_address: ManagedAddress,
    ) -> subscription_fee::Proxy<Self::Api>;

    #[storage_mapper("subtractUserIndex")]
    fn subtract_user_index(&self) -> SingleValueMapper<usize>;

    #[storage_mapper("nextSubtractEpoch")]
    fn next_subtract_epoch(&self, service_index: usize) -> SingleValueMapper<Epoch>;

    #[storage_mapper("userFees")]
    fn user_fees(
        &self,
        service_index: usize,
        user_id: AddressId,
    ) -> SingleValueMapper<UserFees<Self::Api>>;
}
