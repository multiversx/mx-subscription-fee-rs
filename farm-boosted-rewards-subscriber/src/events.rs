multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[derive(ManagedVecItem, TypeAbi, TopEncode, NestedEncode)]
pub struct ClaimRewardsOperation<M: ManagedTypeApi> {
    user: ManagedAddress<M>,
    farm_ids: ManagedVec<M, AddressId>,
}

impl<M: ManagedTypeApi> ClaimRewardsOperation<M> {
    pub fn new(user: ManagedAddress<M>, farm_ids: ManagedVec<M, AddressId>) -> Self {
        ClaimRewardsOperation { user, farm_ids }
    }
}

#[multiversx_sc::module]
pub trait EventsModule {
    fn emit_claim_rewards_event(
        self,
        claim_rewards_operations: ManagedVec<ClaimRewardsOperation<Self::Api>>,
    ) {
        let caller = self.blockchain().get_caller();
        let epoch = self.blockchain().get_block_epoch();
        self.claim_rewards_event(caller, epoch, claim_rewards_operations)
    }

    fn emit_subtract_payment_event(self, service_index: usize, user_ids: ManagedVec<AddressId>) {
        let caller = self.blockchain().get_caller();
        let epoch = self.blockchain().get_block_epoch();
        self.subtract_payment_event(caller, epoch, service_index, user_ids)
    }

    fn emit_mex_operation_event(self, service_index: usize, user_ids: ManagedVec<AddressId>) {
        let caller = self.blockchain().get_caller();
        let epoch = self.blockchain().get_block_epoch();
        self.mex_operation_event(caller, epoch, service_index, user_ids)
    }

    #[event("claimRewardsEvent")]
    fn claim_rewards_event(
        self,
        #[indexed] caller: ManagedAddress,
        #[indexed] epoch: u64,
        claim_rewards_operations: ManagedVec<ClaimRewardsOperation<Self::Api>>,
    );

    #[event("subtractPaymentEvent")]
    fn subtract_payment_event(
        self,
        #[indexed] caller: ManagedAddress,
        #[indexed] epoch: u64,
        #[indexed] service_index: usize,
        user_ids: ManagedVec<AddressId>,
    );

    #[event("mexOperationEvent")]
    fn mex_operation_event(
        self,
        #[indexed] caller: ManagedAddress,
        #[indexed] epoch: u64,
        #[indexed] service_index: usize,
        user_ids: ManagedVec<AddressId>,
    );
}
