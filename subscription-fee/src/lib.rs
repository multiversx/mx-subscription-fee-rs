#![no_std]

multiversx_sc::imports!();

pub mod daily_operations;
pub mod fees;
pub mod low_level_actions;
pub mod pair_actions;
pub mod service;
pub mod user_tokens;

#[multiversx_sc::contract]
pub trait SubscriptionFee:
    fees::FeesModule
    + service::ServiceModule
    + daily_operations::DailyOperationsModule
    + pair_actions::PairActionsModule
    + low_level_actions::LowLevelActionsModule
    + user_tokens::UserTokensModule
    + multiversx_sc_modules::ongoing_operation::OngoingOperationModule
{
    /// Price query address: The address to gather the token to USDC price
    /// Accepted tokens: The tokens users can deposit for fees
    #[init]
    fn init(
        &self,
        price_query_address: ManagedAddress,
        accepted_tokens: MultiValueEncoded<EgldOrEsdtTokenIdentifier>,
    ) {
        require!(
            self.blockchain().is_smart_contract(&price_query_address),
            "Invalid price query address"
        );

        self.price_query_address().set(price_query_address);
        self.add_accepted_fees_tokens(accepted_tokens);
    }
}
