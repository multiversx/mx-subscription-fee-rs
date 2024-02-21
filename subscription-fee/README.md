# Subscription Fee Smart Contract

The Smart Contract provides a comprehensive set of functionalities for managing subscription services and fees on the MultiversX blockchain. It allows users to register services, subscribe to services, deposit funds, withdraw funds, and perform various other actions related to managing subscription services and fees.

The code consists of multiple modules and endpoints that handle various operations related to subscription services, fees, pair actions, among others.

Let's break down the key components and functionalities of the Smart Contract:

## Initialization

Defines the initialization function for setting up the SC with the mandatory variables.

## Service Module

Manages the registration, approval, and subscription of services. It also defines the structure of a service, including payment information and subscription epochs. The module allows the service provider to register or add extra services, unregister services, and the users to subscribe/unsubscribe to/from those said services.

## Fees Module

Handles the addition of accepted fee tokens, setting minimum deposit values, user deposits, and fund withdrawals.

## Common Storage Module

Contains storage mappers and views for various data storage and retrieval operations used by other modules.

## Pair Actions Module

Deals with pair related operations and price queries. Provides functions for adding/removing pair addresses and retrieving token prices.

## Subtract Payments Module

Handles the subtraction of payments for subscribed services. Uses a custom result type for successful or failed price queries, to have a more flexible code output.
