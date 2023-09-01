#![no_std]

multiversx_sc::imports!();

#[multiversx_sc::contract]
pub trait SubscriptionFee {
    #[init]
    fn init(&self) {}
}
