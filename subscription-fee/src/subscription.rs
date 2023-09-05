use crate::service::SubscriptionType;

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait SubscriptionModule: crate::fees::FeesModule + crate::service::ServiceModule {
    /// subscribe with pair of address, service index, subscription type
    #[endpoint]
    fn subscribe(
        &self,
        services: MultiValueEncoded<MultiValue3<ManagedAddress, usize, SubscriptionType>>,
    ) {
        let caller = self.blockchain().get_caller();
        let caller_id = self.user_ids().get_id_non_zero(&caller);

        for pair in services {
            let (service_address, service_index, subscription_type) = pair.into_tuple();
            let service_id = self.service_id().get_id_non_zero(&service_address);
            let service_options = self.service_info(service_id).get();
            require!(
                service_index < service_options.len(),
                "Invalid service index"
            );

            self.subscription_type(caller_id, service_id, service_index)
                .set(subscription_type);
        }
    }

    #[endpoint]
    fn unsubscribe(&self, services: MultiValueEncoded<MultiValue2<ManagedAddress, usize>>) {
        let caller = self.blockchain().get_caller();
        let caller_id = self.user_ids().get_id_non_zero(&caller);

        for pair in services {
            let (service_address, service_index) = pair.into_tuple();
            let service_id = self.service_id().get_id_non_zero(&service_address);
            let service_options = self.service_info(service_id).get();
            require!(
                service_index < service_options.len(),
                "Invalid service index"
            );

            self.subscription_type(caller_id, service_id, service_index)
                .clear();
        }
    }
}
