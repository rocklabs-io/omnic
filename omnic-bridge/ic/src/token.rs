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
    fn swap(
        &mut self,
        from: Self::AccountItem,
        to: Self::AccountItem,
        value: Self::ValueItem,
    ) -> bool;

    fn balance_of(&self, from: &Self::AccountItem) -> Self::OutputItem;
    fn get_total_supply(&self) -> Self::OutputItem;
}

#[derive(Deserialize, CandidType, Clone, Debug)]
pub struct Token<T: CandidType + std::cmp::Ord> {
    src_chain: u32,
    src_pool_id: Nat,
    name: String,
    symbol: String,
    decimals: u8,
    total_supply: Nat,
    balances: BTreeMap<T, Nat>,
}

// external common interface
impl<T> Token<T>
where
    T: std::fmt::Debug + Clone + candid::CandidType + std::cmp::Ord,
{
    pub fn new(
        src_chain: u32,
        src_pool_id: Nat,
        name: String,
        symbol: String,
        decimals: u8,
        balances: BTreeMap<T, Nat>,
    ) -> Self {
        Token {
            src_chain,
            src_pool_id,
            name,
            symbol,
            decimals,
            total_supply: Nat::from(0),
            balances,
        }
    }

    pub fn src_chain_id(&self) -> u32 {
        self.src_chain.clone()
    }

    pub fn src_chain_pool_id(&self) -> Nat {
        self.src_pool_id.clone()
    }

    pub fn token_name(&self) -> String {
        self.name.to_string()
    }

    pub fn token_sumbol(&self) -> String {
        self.symbol.to_string()
    }

    pub fn decimals(&self) -> u8 {
        self.decimals.clone()
    }
}

//
impl<T> Operation for Token<T>
where
    T: std::fmt::Debug + Clone + candid::CandidType + std::cmp::Ord,
{
    type AccountItem = T;
    type ValueItem = Nat;
    type OutputItem = Nat;

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
        self.balances.get(from).cloned().unwrap_or(Self::OutputItem::from(0u32))
    }
    
    fn swap(
        &mut self,
        from: Self::AccountItem,
        to: Self::AccountItem,
        value: Self::ValueItem,
    ) -> bool {
        //todo
        false
    }

    fn get_total_supply(&self) -> Self::OutputItem {
        self.total_supply.clone()
    }
}
