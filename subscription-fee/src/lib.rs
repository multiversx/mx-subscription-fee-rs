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
        max_user_deposits: usize,
        min_user_deposit_value: BigUint,
        max_service_info_no: usize,
        price_query_address: ManagedAddress,
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
            max_user_deposits > 0,
            "Max user deposits no must be greater than 0"
        );
        require!(
            min_user_deposit_value > 0,
            "Min user deposit value must be greater than 0"
        );
        require!(
            max_service_info_no > 0,
            "Max service info no must be greater than 0"
        );
        require!(
            self.blockchain().is_smart_contract(&price_query_address),
            "Invalid price query address"
        );

        self.stable_token_id().set_if_empty(stable_token_id);
        self.wegld_token_id().set_if_empty(wegld_token_id);
        self.price_query_address().set_if_empty(price_query_address);
        self.add_accepted_fees_tokens(accepted_tokens);
        self.max_user_deposits().set_if_empty(max_user_deposits);
        self.min_user_deposit_value()
            .set_if_empty(min_user_deposit_value);
        self.max_service_info_no().set_if_empty(max_service_info_no);
    }
}
