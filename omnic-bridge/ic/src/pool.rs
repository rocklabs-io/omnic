use candid::{types::number::Nat, CandidType, Deserialize};
use std::collections::BTreeMap;
use std::string::String;

use crate::token::{Operation, Token};

// Pool errors
#[derive(derive_more::Display, Debug, Clone, PartialEq)]
pub enum Error {
    #[display(fmt = "Invalid: {}", _0)]
    Invalid(String),
}

#[derive(Deserialize, CandidType, Clone, Debug)]
pub struct Pool<T>
where
    T: std::fmt::Debug + Clone,
    T: CandidType + std::cmp::Ord,
{
    pool_id: Nat,
    tokens: BTreeMap<u32, Token<T>>, // chain_id -> token
    total_liquidity: Nat,            // sum of tokens supply
}

impl<T> Pool<T>
where
    T: std::fmt::Debug + Clone,
    T: CandidType + std::cmp::Ord,
{
    pub fn new(pool_id: Nat, tokens: BTreeMap<u32, Token<T>>) -> Self {
        Pool {
            pool_id,
            tokens,
            total_liquidity: Nat::from(0),
        }
    }
    pub fn get_pool_id(&self) -> Nat {
        self.pool_id.clone()
    }

    pub fn get_tokens_len(&self) -> Nat {
        Nat::from(self.tokens.len())
    }

    pub fn add_token(&mut self, chain_id: u32, token: Token<T>) -> bool {
        self.tokens.entry(chain_id).or_insert(token);
        true
    }

    pub fn remove_token(&mut self, chain_id: u32) -> Token<T> {
        //
        self.tokens.remove(&chain_id).unwrap()
    }

    pub fn get_token_by_chain_id(&self, chain_id: u32) -> Option<Token<T>> {
        //
        self.tokens.get(&chain_id).cloned()
    }

    pub fn contain_token(&self, chain_id: u32) -> bool {
        //
        self.tokens.contains_key(&chain_id)
    }

    pub fn get_sub_token_supply_by_chain_id(&self, chain_id: u32) -> Nat {
        // TODO: handle unwrap
        let token = self.get_token_by_chain_id(chain_id).unwrap();
        Nat::from(token.get_total_supply())
    }

    pub fn total_liquidity(&self) -> Nat {
        let mut total_liquidity: Nat = Nat::from(0);
        for token in self.tokens.values() {
            total_liquidity += Nat::from(token.get_total_supply());
        }
        total_liquidity
    }
}
