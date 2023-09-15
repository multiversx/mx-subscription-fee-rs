multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait BaseInitModule: crate::common_storage::CommonStorageModule {
    fn base_init(&self, fees_contract_address: ManagedAddress) {
        require!(
            self.blockchain().is_smart_contract(&fees_contract_address),
            "Invalid address"
        );

        self.fees_contract_address().set(fees_contract_address);
    }
}
