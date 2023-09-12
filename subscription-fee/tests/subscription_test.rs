use multiversx_sc::types::Address;
use subscription_setup::SubscriptionSetup;

mod subscription_setup;

#[test]
fn test() {
    let _ = SubscriptionSetup::new(subscription_fee::contract_obj, &Address::zero(), Vec::new());
}
