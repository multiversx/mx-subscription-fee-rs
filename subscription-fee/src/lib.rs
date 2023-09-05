#![no_std]

multiversx_sc::imports!();

pub mod daily_operations;
pub mod fees;
pub mod pair_actions;
pub mod service;
pub mod subscription;

#[multiversx_sc::contract]
pub trait SubscriptionFee:
    fees::FeesModule
    + service::ServiceModule
    + subscription::SubscriptionModule
    + daily_operations::DailyOperationsModule
    + pair_actions::PairActionsModule
    + energy_query::EnergyQueryModule
    + multiversx_sc_modules::ongoing_operation::OngoingOperationModule
{
    /// Energy threshold: Used to determine either normal or premium account
    /// Energy query: The address from which the energy is queried
    /// Price query address: The address to gather the token to USDC price
    /// Accepted tokens: The tokens users can deposit for fees
    #[init]
    fn init(
        &self,
        energy_threshold: BigUint,
        energy_query_address: ManagedAddress,
        price_query_address: ManagedAddress,
        accepted_tokens: MultiValueEncoded<EgldOrEsdtTokenIdentifier>,
    ) {
        require!(
            self.blockchain().is_smart_contract(&price_query_address),
            "Invalid price query address"
        );

        self.energy_threshold().set(energy_threshold);
        self.price_query_address().set(price_query_address);

        self.set_energy_factory_address(energy_query_address);
        self.add_accepted_fees_tokens(accepted_tokens);
    }
}
