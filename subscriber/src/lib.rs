#![no_std]
#![feature(trait_alias)]

multiversx_sc::imports!();

pub mod base_functions;
pub mod base_init;
pub mod common_storage;
pub mod daily_operations;
pub mod service;
pub mod subscription;
pub mod user_tokens;

#[multiversx_sc::contract]
pub trait SubscriberContractMain:
    base_init::BaseInitModule
    + service::ServiceModule
    + daily_operations::DailyOperationsModule
    + user_tokens::UserTokensModule
    + subscription::SubscriptionModule
    + common_storage::CommonStorageModule
    + energy_query::EnergyQueryModule
    + multiversx_sc_modules::ongoing_operation::OngoingOperationModule
{
    #[init]
    fn init(
        &self,
        fees_contract_address: ManagedAddress,
        accepted_tokens: MultiValueEncoded<TokenIdentifier>,
    ) {
        self.base_init(fees_contract_address, accepted_tokens);
    }
}
