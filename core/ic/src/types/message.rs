
use candid::Deserialize;
use ic_web3::types::H256;
use crate::{utils::keccak256, Decode, Encode, OmnicError};

const PREFIX_LEN: usize = 77;

#[derive(Debug, Default, Clone, Deserialize)]
pub struct Message {
    /// 4   SLIP-44 ID
    pub origin: u32,
    /// 32  Address in home convention
    pub sender: H256,
    /// 4   Count of all previous messages to destination
    pub nonce: u32,
    /// 4   SLIP-44 ID
    pub destination: u32,
    /// 32  Address in destination convention
    pub recipient: H256,
    /// 1   Wait for optimistic verification period
    pub wait_optimistic: bool,
    /// 0+  Message contents
    pub body: Vec<u8>,
}

/// A partial Nomad message between chains
#[derive(Debug, Default, Clone)]
pub struct MessageBody {
    /// 4   SLIP-44 ID
    pub destination: u32,
    /// 32  Address in destination convention
    pub recipient: H256,
    /// 0+  Message contents
    pub body: Vec<u8>,
}

impl Encode for Message {
    fn write_to<W>(&self, writer: &mut W) -> std::io::Result<usize>
    where
        W: std::io::Write,
    {
        writer.write_all(&self.origin.to_be_bytes())?;
        writer.write_all(self.sender.as_ref())?;
        writer.write_all(&self.nonce.to_be_bytes())?;
        writer.write_all(&self.destination.to_be_bytes())?;
        writer.write_all(self.recipient.as_ref())?;
        let v = if self.wait_optimistic { 1u8 } else { 0u8 };
        writer.write_all(&v.to_be_bytes())?;
        writer.write_all(&self.body)?;
        Ok(PREFIX_LEN + self.body.len())
    }
}

impl Decode for Message {
    fn read_from<R>(reader: &mut R) -> Result<Self, OmnicError>
    where
        R: std::io::Read,
    {
        let mut origin = [0u8; 4];
        reader.read_exact(&mut origin)?;

        let mut sender = H256::zero();
        reader.read_exact(sender.as_mut())?;

        let mut nonce = [0u8; 4];
        reader.read_exact(&mut nonce)?;

        let mut destination = [0u8; 4];
        reader.read_exact(&mut destination)?;

        let mut recipient = H256::zero();
        reader.read_exact(recipient.as_mut())?;

        let mut v = [0u8; 1];
        reader.read_exact(&mut v)?;
        let wait_optimistic = v[0] == 1; //u32::from_be_bytes(v) == 1;

        let mut body = vec![];
        reader.read_to_end(&mut body)?;

        Ok(Self {
            origin: u32::from_be_bytes(origin),
            sender,
            destination: u32::from_be_bytes(destination),
            recipient,
            nonce: u32::from_be_bytes(nonce),
            wait_optimistic,
            body,
        })
    }
}

impl Message {
    /// Convert the message to a leaf
    pub fn to_leaf(&self) -> H256 {
        keccak256(&self.to_vec()).into()
    }
}

impl std::fmt::Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Message {}->{}:{}",
            self.origin, self.destination, self.nonce,
        )
    }
}