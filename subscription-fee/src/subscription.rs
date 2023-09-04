multiversx_sc::imports!();

pub const DAILY_EPOCHS: u64 = 1;
pub const WEEKLY_EPOCHS: u64 = 7;
pub const MONTHLY_EPOCHS: u64 = 30;

pub enum SubscriptionType {
    Daily,
    Weekly,
    Monthly,
}

#[multiversx_sc::module]
pub trait SubscriptionModule {
    #[endpoint]
    fn subscribe(&self) {}
}
