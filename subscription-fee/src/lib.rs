#![no_std]

multiversx_sc::imports!();

pub mod common_storage;
pub mod fees;
pub mod pair_actions;
pub mod service;
pub mod subtract_payments;

#[multiversx_sc::contract]
pub trait SubscriptionFee:
    fees::FeesModule
    + common_storage::CommonStorageModule
    + service::ServiceModule
    + subtract_payments::SubtractPaymentsModule
    + pair_actions::PairActionsModule
    + multiversx_sc_modules::ongoing_operation::OngoingOperationModule
{
    /// Price query address: The address to gather the token to USDC price
    /// Accepted tokens: The tokens users can deposit for fees
    #[init]
    fn init(
        &self,
        stable_token_id: TokenIdentifier,
        wegld_token_id: TokenIdentifier,
        price_query_address: ManagedAddress,
        min_stable_token_deposit_value: BigUint,
        accepted_tokens: MultiValueEncoded<TokenIdentifier>,
    ) {
        require!(
            stable_token_id.is_valid_esdt_identifier(),
            "Stable token not valid"
        );
        require!(
            wegld_token_id.is_valid_esdt_identifier(),
            "WEGLD token not valid"
        );
        require!(
            self.blockchain().is_smart_contract(&price_query_address),
            "Invalid price query address"
        );
        require!(
            min_stable_token_deposit_value > 0,
            "Min stable token deposit value must be greater than 0"
        );

        self.stable_token_id().set_if_empty(stable_token_id);
        self.wegld_token_id().set_if_empty(wegld_token_id);
        self.price_query_address().set_if_empty(price_query_address);
        self.min_stable_token_deposit_value()
            .set_if_empty(min_stable_token_deposit_value);
        self.add_accepted_fees_tokens(accepted_tokens);
    }

    #[upgrade]
    fn upgrade(&self) {}
}
