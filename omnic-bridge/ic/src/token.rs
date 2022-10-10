use candid::{types::number::Nat, CandidType, Deserialize};
use std::collections::BTreeMap;
use std::convert::From;
use std::string::String;

// Token errors
#[derive(derive_more::Display, Debug, Clone, PartialEq)]
pub enum Error {
    #[display(fmt = "Invalid: {}", _0)]
    Invalid(String),

    #[display(fmt = "operation: {} failed!", _0)]
    Operation(String),
}

pub trait Operation: std::fmt::Debug + Clone {
    type AccountItem;
    type ValueItem;
    type OutputItem;

    fn burn(&mut self, from: Self::AccountItem, value: Self::ValueItem) -> bool;
    fn mint(&mut self, to: Self::AccountItem, value: Self::ValueItem) -> bool;

    fn balance_of(&self, from: &Self::AccountItem) -> Self::OutputItem;
    fn total_supply(&self) -> Self::OutputItem;
}

#[derive(Deserialize, CandidType, Clone, Debug)]
pub struct Token {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub address: String,
}

// external common interface
impl Token {
    pub fn new(
        name: String,
        symbol: String,
        decimals: u8,
        address: String,
    ) -> Self {
        Token {
            name,
            symbol,
            decimals,
            address,
        }
    }
}
