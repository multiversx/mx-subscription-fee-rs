#![no_std]

multiversx_sc::imports!();

mod fees;
mod service;
mod subscription;

#[multiversx_sc::contract]
pub trait SubscriptionFee:
    fees::FeesModule + service::ServiceModule + subscription::SubscriptionModule
{
    #[init]
    fn init(&self, accepted_tokens: MultiValueEncoded<EgldOrEsdtTokenIdentifier>) {
        self.add_accepted_fees_tokens(accepted_tokens);
    }
}
