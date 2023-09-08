use crate::service::SubscriptionType;

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait SubscriptionModule:
    crate::service::ServiceModule + crate::common_storage::CommonStorageModule
{
    /// subscribe with pair of service index, subscription type
    #[endpoint]
    fn subscribe(&self, services: MultiValueEncoded<MultiValue2<usize, SubscriptionType>>) {
        let fees_contract_address = self.fees_contract_address().get();
        let caller = self.blockchain().get_caller();
        let caller_id = self
            .user_id()
            .get_id_at_address_non_zero(&fees_contract_address, &caller);

        for pair in services {
            let (service_index, subscription_type) = pair.into_tuple();
            let service_options = self.service_info().get();
            require!(
                service_index < service_options.len(),
                "Invalid service index"
            );

            self.subscription_type(caller_id, service_index)
                .set(subscription_type);
        }
    }

    // unsubscribe from the given service indexes
    #[endpoint]
    fn unsubscribe(&self, service_indexes: MultiValueEncoded<usize>) {
        let fees_contract_address = self.fees_contract_address().get();
        let caller = self.blockchain().get_caller();
        let caller_id = self
            .user_id()
            .get_id_at_address_non_zero(&fees_contract_address, &caller);

        for service_index in service_indexes {
            let service_options = self.service_info().get();
            require!(
                service_index < service_options.len(),
                "Invalid service index"
            );

            self.subscription_type(caller_id, service_index).clear();
        }
    }
}
