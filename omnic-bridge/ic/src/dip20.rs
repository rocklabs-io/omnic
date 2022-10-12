use candid::{CandidType, Deserialize, types::number::Nat, types::principal::Principal};

#[allow(non_snake_case)]
#[derive(Deserialize, CandidType, Clone, Debug)]
pub struct Metadata {
  pub logo: String,
  pub name: String,
  pub symbol: String,
  pub decimals: u8,
  pub totalSupply: Nat,
  pub owner: Principal,
  pub fee: Nat,
}

#[derive(CandidType, Deserialize, Debug, PartialEq)]
pub enum TxError {
    InsufficientBalance,
    InsufficientAllowance,
    Unauthorized,
    LedgerTrap,
    AmountTooSmall,
    BlockUsed,
    ErrorOperationStyle,
    ErrorTo,
    Other(String),
}
pub type TxReceipt = std::result::Result<Nat, TxError>;