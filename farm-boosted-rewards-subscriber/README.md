# Farm Boosted Subscriber SC

__The Farm Boosted Subscriber SC__ handles operations such as subtracting payments, claiming fees, performing MEX operations, adding/removing farms, and managing subscriber configurations, by interacting with the __Subscription Fee SC__.

It consists of multiple modules and traits, each containing various endpoint functions and event emissions. Here's a breakdown of the main components:

## Initialization

Contains initialization, upgrade, and other endpoint functions related to setting configurations and performing various operations.

## Service Module

Defines a MexOperationItem struct representing a user address and an amount.
Implements various endpoints for subtracting payments, claiming fees, and performing MEX operations.

## Events Module

Defines the EventsModule trait responsible for emitting different events such as claim_rewards_event, subtract_payment_event, and mex_operation_event.

## ClaimFarmBoostedRewards Module

Defines the ClaimFarmBoostedRewardsModule trait with functions to add and remove farms, and perform claim rewards operations.

Implements a module for claiming farm boosted rewards.
Defines endpoints for adding/removing farms and performing claim rewards operations.

## SubscriberConfig Module

Contains the SubscriberConfigModule trait with functions for handling various configurations and actions related to subscriber settings, such as percentages, epochs, and subscription user types.
Defines functions for setting fees claim address, adding token max fee withdraw per week, and other utility functions.
