use subscription_setup::SubscriptionSetup;

mod subscription_setup;

#[test]
fn test() {
    let _ = SubscriptionSetup::new(subscription_fee::contract_obj);
}
