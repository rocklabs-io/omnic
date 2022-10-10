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
    pub src_chain: u32,
    pub src_pool_id: u32,
    pub src_addr: String,
    pub name: String,
    pub symbol: String,
    pub local_decimals: u8,
    pub shared_decimals: u8,
    pub total_supply: u128,
    pub balances: BTreeMap<String, u128>, // shared_decimals token
}

// external common interface
impl Token {
    pub fn new(
        src_chain: u32,
        src_pool_id: u32,
        name: String,
        symbol: String,
        local_decimals: u8,
        shared_decimals: u8,
        balances: BTreeMap<String, u128>,
    ) -> Self {
        Token {
            src_chain,
            src_pool_id,
            name,
            symbol,
            local_decimals,
            shared_decimals,
            total_supply: 0u128,
            balances,
        }
    }
}

//
impl Operation for Token {
    type AccountItem = String;
    type ValueItem = u128;
    type OutputItem = u128;

    fn mint(&mut self, to: Self::AccountItem, value: Self::ValueItem) -> bool {
        let amount = self.balance_of(&to) + value.clone();
        self.balances.insert(to, amount);
        self.total_supply += value;
        true
    }

    fn burn(&mut self, from: Self::AccountItem, value: Self::ValueItem) -> bool {
        if self.balance_of(&from) < value.clone() {
            return false;
        }
        let amount = self.balance_of(&from) - value.clone();
        self.balances.insert(from, amount);
        self.total_supply -= value;
        true
    }

    fn balance_of(&self, from: &Self::AccountItem) -> Self::OutputItem {
        self.balances.get(from).cloned().unwrap_or(Self::OutputItem::from(0))
    }

    fn get_total_supply(&self) -> Self::OutputItem {
        self.total_supply.clone()
    }
}
