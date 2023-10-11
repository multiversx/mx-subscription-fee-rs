multiversx_sc::imports!();

use auto_farm::common::{
    address_to_id_mapper::{AddressId, AddressToIdMapper},
    unique_payments::UniquePayments,
};

use crate::{service::ServiceInfo, subtract_payments::Epoch};

#[multiversx_sc::module]
pub trait CommonStorageModule {
    #[view(getAcceptedFeesTokens)]
    #[storage_mapper("acceptedFeesTokens")]
    fn accepted_fees_tokens(&self) -> UnorderedSetMapper<TokenIdentifier>;

    #[storage_mapper("userId")]
    fn user_id(&self) -> AddressToIdMapper<Self::Api>;

    #[view(getUserDepositedFees)]
    #[storage_mapper("userDepositedFees")]
    fn user_deposited_fees(
        &self,
        user_id: AddressId,
    ) -> SingleValueMapper<UniquePayments<Self::Api>>;

    #[view(getMaxUserDeposits)]
    #[storage_mapper("maxUserDeposits")]
    fn max_user_deposits(&self) -> SingleValueMapper<usize>;

    #[view(getMinUserDepositValue)]
    #[storage_mapper("minUserDepositValue")]
    fn min_user_deposit_value(&self) -> SingleValueMapper<BigUint>;

    #[storage_mapper("userLastActionEpoch")]
    fn user_last_action_epoch(
        &self,
        user_id: AddressId,
        service_id: AddressId,
        service_index: usize,
    ) -> SingleValueMapper<Epoch>;

    #[storage_mapper("serviceId")]
    fn service_id(&self) -> AddressToIdMapper<Self::Api>;

    #[view(getPendingServices)]
    #[storage_mapper("pendingServices")]
    fn pending_services(&self) -> UnorderedSetMapper<ManagedAddress>;

    #[view(getMaxPendingServices)]
    #[storage_mapper("maxPendingServices")]
    fn max_pending_services(&self) -> SingleValueMapper<usize>;

    #[storage_mapper("pendingServiceInfo")]
    fn pending_service_info(
        &self,
        service_address: &ManagedAddress,
    ) -> SingleValueMapper<ManagedVec<ServiceInfo<Self::Api>>>;

    // one service may have multiple options
    #[view(getServiceInfo)]
    #[storage_mapper("serviceInfo")]
    fn service_info(
        &self,
        service_id: AddressId,
    ) -> SingleValueMapper<ManagedVec<ServiceInfo<Self::Api>>>;

    #[storage_mapper("maxServiceInfoNo")]
    fn max_service_info_no(&self) -> SingleValueMapper<usize>;

    #[view(getSubscribedUsers)]
    #[storage_mapper("subscribedUsers")]
    fn subscribed_users(
        &self,
        service_id: AddressId,
        service_index: usize,
    ) -> UnorderedSetMapper<AddressId>;

    #[storage_mapper("pairAddressForToken")]
    fn pair_address_for_token(
        &self,
        token_id: &TokenIdentifier,
    ) -> SingleValueMapper<ManagedAddress<Self::Api>>;

    #[storage_mapper("stableTokenId")]
    fn stable_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[storage_mapper("wegldTokenId")]
    fn wegld_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[storage_mapper("priceQueryAddress")]
    fn price_query_address(&self) -> SingleValueMapper<ManagedAddress>;
}
