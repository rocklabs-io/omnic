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

// impl Operation for Token {
//     type AccountItem = String;
//     type ValueItem = u128;
//     type OutputItem = u128;

//     fn mint(&mut self, to: Self::AccountItem, value: Self::ValueItem) -> bool {
//         let amount = self.balance_of(&to) + value.clone();
//         self.balances.insert(to, amount);
//         self.total_supply += value;
//         true
//     }

//     fn burn(&mut self, from: Self::AccountItem, value: Self::ValueItem) -> bool {
//         if self.balance_of(&from) < value.clone() {
//             return false;
//         }
//         let amount = self.balance_of(&from) - value.clone();
//         self.balances.insert(from, amount);
//         self.total_supply -= value;
//         true
//     }

//     fn balance_of(&self, from: &Self::AccountItem) -> Self::OutputItem {
//         self.balances.get(from).cloned().unwrap_or(Self::OutputItem::from(0))
//     }

//     fn get_total_supply(&self) -> Self::OutputItem {
//         self.total_supply.clone()
//     }
// }
