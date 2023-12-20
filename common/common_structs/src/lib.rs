#![no_std]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub mod unique_payments;

pub use unique_payments::*;
