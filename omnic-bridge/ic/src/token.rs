use candid::{candid_method, CandidType, Deserialize, types::number::Nat};
use ic_cdk_macros::*;
use std::collections::{HashMap, BTreeMap, VecDeque};
use std::iter::FromIterator;
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
    fn burn(&mut self, from: Vec<u8>, value: Nat) -> bool;
    fn mint(&mut self, to: Vec<u8>, value: Nat) -> bool;
    fn balanceOf(&self, from: &[u8]) -> Nat; 
    fn swap(&mut self, from: Vec<u8>, to: Vec<u8>, value: Nat) -> bool;

    fn totalSupply(&self) -> Nat;

}


#[derive(Deserialize, CandidType, Clone, Debug)]
pub struct Token {
    src_chain: Nat,
    src_pool_id: Nat,
    name: String,
    symbol: String,
    decimals: u8,
    total_supply: Nat,
    balances: BTreeMap<Vec<u8>, Nat>,
}

impl Token {
    fn new(src_chain: Nat, src_pool_id: Nat, name: String, symbol: String, decimals: u8) -> Self {
        Token {
            src_chain,
            src_pool_id,
            name,
            symbol,
            decimals,
            total_supply: Nat::from(0),
            balances: BTreeMap::new(),
        }
    }

    pub fn srcChainId(&self) -> Nat {
        self.src_chain.clone()
    }

    pub fn tokenName(&self) -> String {
        String::from(&self.name)
    }

    pub fn decimals(&self) -> u8 {
        self.decimals.clone()
    }

    pub fn totalSupply(&self) -> Nat {
        self.total_supply.clone()
    }
}

impl Operation for Token {

    fn mint(&mut self, to: Vec<u8>, value: Nat) -> bool {
        let amount: Nat = self.balanceOf(&to) + value.clone();
        self.balances.insert(to, amount);
        self.total_supply += value;
        true
    }

    fn burn(&mut self, from: Vec<u8>, value: Nat) -> bool {
        if self.balanceOf(&from) < value.clone() {
            return false;
        }
        let amount: Nat = self.balanceOf(&from) - value.clone();
        self.balances.insert(from, amount);
        self.total_supply -= value;
        true
    }

    fn balanceOf(&self, from: &[u8]) -> Nat {
        self.balances.get(from).cloned().unwrap_or(Nat::from(0))
    }

    fn swap(&mut self, from: Vec<u8>, to: Vec<u8>, value: Nat) -> bool {
        //todo
        false
    }

    fn totalSupply(&self) -> Nat {
        self.total_supply.clone()
    }


}