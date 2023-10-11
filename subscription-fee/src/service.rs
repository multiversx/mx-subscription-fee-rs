multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use auto_farm::common::address_to_id_mapper::{AddressId, NULL_ID};

use crate::common_storage;
use crate::subtract_payments::Epoch;
use crate::{fees, pair_actions};

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, ManagedVecItem)]
pub struct ServiceInfo<M: ManagedTypeApi> {
    pub opt_payment_token: Option<TokenIdentifier<M>>,
    pub amount: BigUint<M>,
    pub subscription_epochs: Epoch,
}

#[multiversx_sc::module]
pub trait ServiceModule:
    fees::FeesModule + pair_actions::PairActionsModule + common_storage::CommonStorageModule
{
    #[only_owner]
    #[endpoint(setMaxServiceInfoNo)]
    fn set_max_service_info_no(&self, max_service_info_no: usize) {
        require!(max_service_info_no > 0, "Value must be greater than o");
        self.max_service_info_no().set(max_service_info_no);
    }

    #[only_owner]
    #[endpoint(setMaxPendingServices)]
    fn set_max_pending_services(&self, max_pending_services: usize) {
        require!(max_pending_services > 0, "Value must be greater than o");
        self.max_pending_services().set(max_pending_services);
    }

    /// Arguments are MultiValue3 of opt_payment_token, payment_amount and subscription_epochs
    #[endpoint(registerService)]
    fn register_service(
        &self,
        args: MultiValueEncoded<MultiValue3<Option<TokenIdentifier>, BigUint, Epoch>>,
    ) {
        require!(!args.is_empty(), "No arguments provided");

        let service_address = self.blockchain().get_caller();
        let existing_service_id = self.service_id().get_id(&service_address);
        require!(existing_service_id == NULL_ID, "Service already registered");

        let mut services = ManagedVec::<Self::Api, _>::new();
        for arg in args {
            let (opt_payment_token, amount, subscription_epochs) = arg.into_tuple();

            if let Some(token_id) = &opt_payment_token {
                require!(
                    self.accepted_fees_tokens().contains(token_id),
                    "Invalid token ID"
                );
            }

            services.push(ServiceInfo {
                opt_payment_token,
                amount,
                subscription_epochs,
            });
        }

        self.pending_service_info(&service_address)
            .update(|existing_services| existing_services.extend(services.iter()));
        let _ = self.pending_services().insert(service_address);
        let max_pending_services = self.max_pending_services().get();
        require!(
            self.pending_services().len() <= max_pending_services,
            "Maximum number of pendind services reached"
        );
    }

    #[endpoint(addExtraServices)]
    fn add_extra_services(
        &self,
        args: MultiValueEncoded<MultiValue3<Option<TokenIdentifier>, BigUint, Epoch>>,
    ) {
        require!(!args.is_empty(), "No arguments provided");

        let service_address = self.blockchain().get_caller();
        let existing_service_id = self.service_id().get_id(&service_address);
        require!(existing_service_id != NULL_ID, "Service not registered");

        let mut services = ManagedVec::<Self::Api, _>::new();
        for arg in args {
            let (opt_payment_token, amount, subscription_epochs) = arg.into_tuple();

            if let Some(token_id) = &opt_payment_token {
                require!(
                    self.accepted_fees_tokens().contains(token_id),
                    "Invalid token ID"
                );
            }

            services.push(ServiceInfo {
                opt_payment_token,
                amount,
                subscription_epochs,
            });
        }

        let service_info_mapper = self.service_info(existing_service_id);
        service_info_mapper.update(|existing_services| existing_services.extend(services.iter()));

        let max_service_info_no = self.max_service_info_no().get();
        require!(
            service_info_mapper.get().len() <= max_service_info_no,
            "Maximum service info no reached"
        );
    }

    #[endpoint(unregisterService)]
    fn unregister_service(&self) {
        let service_address = self.blockchain().get_caller();
        let service_id = self.service_id().remove_by_address(&service_address);
        if service_id != NULL_ID {
            self.service_info(service_id).clear();
        }

        let _ = self.pending_services().swap_remove(&service_address);
        self.pending_service_info(&service_address).clear();
    }

    #[only_owner]
    #[endpoint(unregisterServiceByOwner)]
    fn unregister_service_by_owner(&self, service_address: ManagedAddress) {
        let service_id = self.service_id().remove_by_address(&service_address);
        if service_id != NULL_ID {
            self.service_info(service_id).clear();
        }

        let _ = self.pending_services().swap_remove(&service_address);
        self.pending_service_info(&service_address).clear();
    }

    #[only_owner]
    #[endpoint(approveService)]
    fn approve_service(&self, service_address: ManagedAddress) {
        require!(
            self.pending_services().contains(&service_address),
            "Unknown service"
        );

        let service_id = self.service_id().insert_new(&service_address);
        let service_info = self.pending_service_info(&service_address).take();
        self.service_info(service_id).set(&service_info);

        let max_service_info_no = self.max_service_info_no().get();
        require!(
            self.service_info(service_id).get().len() <= max_service_info_no,
            "Maximum service info no reached"
        );

        let _ = self.pending_services().swap_remove(&service_address);
    }

    /// subscribe with the following arguments: service_id, service index, subscription type
    #[endpoint]
    fn subscribe(&self, services: MultiValueEncoded<MultiValue2<AddressId, usize>>) {
        let caller = self.blockchain().get_caller();
        let caller_id = self.user_id().get_id_non_zero(&caller);

        for service in services {
            let (service_id, service_index) = service.into_tuple();
            let service_options = self.service_info(service_id).get();
            require!(
                service_index < service_options.len(),
                "Invalid service index"
            );

            let _ = self
                .subscribed_users(service_id, service_index)
                .insert(caller_id);
        }
    }

    /// unsubscribe from the given services, by providing the service_id and service indexes
    #[endpoint]
    fn unsubscribe(&self, services: MultiValueEncoded<MultiValue2<AddressId, usize>>) {
        let caller = self.blockchain().get_caller();
        let caller_id = self.user_id().get_id_non_zero(&caller);

        for service in services {
            let (service_id, service_index) = service.into_tuple();
            let service_options = self.service_info(service_id).get();
            require!(
                service_index < service_options.len(),
                "Invalid service index"
            );

            let _ = self
                .subscribed_users(service_id, service_index)
                .swap_remove(&caller_id);
            self.user_last_action_epoch(caller_id, service_id, service_index)
                .clear();
        }
    }
}
