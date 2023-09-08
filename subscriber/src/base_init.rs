multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait BaseInitModule:
    crate::user_tokens::UserTokensModule + crate::common_storage::CommonStorageModule
{
    fn base_init(
        &self,
        fees_contract_address: ManagedAddress,
        accepted_tokens: MultiValueEncoded<TokenIdentifier>,
    ) {
        require!(
            self.blockchain().is_smart_contract(&fees_contract_address),
            "Invalid address"
        );

        self.fees_contract_address().set(fees_contract_address);
        self.add_accepted_user_tokens(accepted_tokens);
    }
}
