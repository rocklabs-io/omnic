use candid::{candid_method, CandidType, Deserialize, types::number::Nat};
use ic_kit::{ic , Principal};
use ic_cdk_macros::*;
use std::collections::{HashMap, BTreeSet, VecDeque};
use std::iter::FromIterator;
use std::string::String;


pub trait Operation: std::fmt::Debug + Clone {
    fn mint(&mut self, to: Vec<u8>, value: Mat) -> bool;
    fn burn(&mut self, from: Vec<u8>, value: Mat) -> bool;
    fn balanceOf(&self, user: Vec<u8>) -> Nat; 
    fn swap(&mut self, from: Vec<u8>, to: Vec<u8>, value: Mat) -> bool;

}


#[derive(Deserialize, CandidType, Clone, Debug)]
pub struct Token {
    src_chain: Nat,
    name: String,
    symbol: String,
    decimals: u8,
    total_supply: Nat,
    balances: BTreeSet<Vec<u8>, Nat>,
}

impl Token {
    fn new(src_chain: Nat, name: String, symbol: String, decimals: u8) -> Self {
        Token {
            src_chain,
            name,
            symbol,
            decimals,
            total_supply: 0.to(),
            balances: BTreeSet::new(),
        }
    }

    fn srcChainId(&self) -> Nat {
        self.src_chain.clone()
    }

    fn tokenName(&self) -> String {
        String::from(&self.name)
    }

    fn decimals(&self) -> u8 {
        self.decimals.clone()
    }

    fn totalSupply(&self) -> Nat {
        self.total_supply.clone()
    }
}

impl Operation for Token {

    pub fn mint(&mut self, to: Vec<u8>, value: Nat) -> bool {
        if(!self.balances.insert(to, self.balanceOf(&to) + value.clone())) {
            return false;
        }
        self.total_supply += value;
        true
    }

    pub fn burn(&mut self, from: Vec<u8>, value: Nat) -> bool {
        if self.balanceOf(&from) < value.clone() {
            return false;
        }
        if(!self.balances.insert(to, self.balanceOf(&to) - value.clone())) {
            return false;
        }
        self.total_supply -= value;
        true
    }

    pub fn balanceOf(&self, account: &Vec<u8>) -> Nat {
        self.balances.get(account).cloned().unwrap_or(0.to())
    }

    pub fn swap(&self, from: &Vec<u8>, to: &Vec<u8>) -> bool {
        //todo
        false
    }


    // internal func


}