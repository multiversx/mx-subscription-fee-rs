#![no_std]

multiversx_sc::imports!();

mod fees;

#[multiversx_sc::contract]
pub trait SubscriptionFee: fees::FeesModule {
    #[init]
    fn init(&self, accepted_tokens: MultiValueEncoded<EgldOrEsdtTokenIdentifier>) {
        self.add_accepted_fees_tokens(accepted_tokens);
    }
}
