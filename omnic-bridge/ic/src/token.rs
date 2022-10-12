/**
 * @brief The basic Metadata of Token
 */

use candid::{CandidType, Deserialize};

#[derive(Deserialize, CandidType, Clone, Debug)]
pub struct Token {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub address: String,
}

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

    pub fn name(&self) -> String {
        self.name.to_string()
    }

    pub fn symbol(&self) -> String {
        self.symbol.to_string()
    }

    pub fn decimals(&self) -> u8 {
        self.decimals.clone()
    }
    
    pub fn address(&self) -> String {
        self.address.clone()
    }
}
