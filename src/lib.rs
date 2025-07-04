use hex;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::Deref;

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct CompactSize {
    pub value: u64,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum BitcoinError {
    InsufficientBytes,
    InvalidFormat,
}

impl CompactSize {
    pub fn new(value: u64) -> Self {
        Self { value }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        match self.value {
            0..=0xFC => {
                bytes.push(self.value as u8);
            }
            0xFD..=0xFFFF => {
                bytes.push(0xFD);
                bytes.extend_from_slice(&(self.value as u16).to_le_bytes());
            }
            0x10000..=0xFFFFFFFF => {
                bytes.push(0xFE);
                bytes.extend_from_slice(&(self.value as u32).to_le_bytes());
            }
            _ => {
                bytes.push(0xFF);
                bytes.extend_from_slice(&self.value.to_le_bytes());
            }
        }
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<(Self, usize), BitcoinError> {
        if bytes.is_empty() {
            return Err(BitcoinError::InsufficientBytes);
        }

        let first_byte = bytes[0];
        match first_byte {
            0..=0xFC => Ok((Self::new(first_byte as u64), 1)),
            0xFD => {
                if bytes.len() < 3 {
                    return Err(BitcoinError::InsufficientBytes);
                }
                let value = u16::from_le_bytes([bytes[1], bytes[2]]) as u64;
                Ok((Self::new(value), 3))
            }
            0xFE => {
                if bytes.len() < 5 {
                    return Err(BitcoinError::InsufficientBytes);
                }
                let value = u32::from_le_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]) as u64;
                Ok((Self::new(value), 5))
            }
            0xFF => {
                if bytes.len() < 9 {
                    return Err(BitcoinError::InsufficientBytes);
                }
                let value = u64::from_le_bytes([
                    bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7], bytes[8],
                ]);
                Ok((Self::new(value), 9))
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Txid(pub [u8; 32]);

impl Serialize for Txid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let hex_string = hex::encode(&self.0);
        serializer.serialize_str(&hex_string)
    }
}

impl<'de> Deserialize<'de> for Txid {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;
        let s: String = Deserialize::deserialize(deserializer)?;
        let bytes = hex::decode(&s).map_err(|_| D::Error::custom("Invalid hex string"))?;

        if bytes.len() != 32 {
            return Err(D::Error::custom("Txid must be exactly 32 bytes"));
        }

        let mut array = [0u8; 32];
        array.copy_from_slice(&bytes);
        Ok(Txid(array))
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct OutPoint {
    pub txid: Txid,
    pub vout: u32,
}

impl OutPoint {
    pub fn new(txid: [u8; 32], vout: u32) -> Self {
        Self {
            txid: Txid(txid),
            vout,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(36);
        bytes.extend_from_slice(&self.txid.0);
        bytes.extend_from_slice(&self.vout.to_le_bytes());
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<(Self, usize), BitcoinError> {
        if bytes.len() < 36 {
            return Err(BitcoinError::InsufficientBytes);
        }

        let mut txid = [0u8; 32];
        txid.copy_from_slice(&bytes[0..32]);

        let mut vout_bytes = [0u8; 4];
        vout_bytes.copy_from_slice(&bytes[32..36]);
        let vout = u32::from_le_bytes(vout_bytes);

        Ok((OutPoint::new(txid, vout), 36))
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Script {
    pub bytes: Vec<u8>,
}

impl Script {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut result = Vec::new();
        let compact_size = CompactSize::new(self.bytes.len() as u64);
        result.extend_from_slice(&compact_size.to_bytes());
        result.extend_from_slice(&self.bytes);
        result
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<(Self, usize), BitcoinError> {
        let (compact_size, size_len) = CompactSize::from_bytes(bytes)?;
        let script_len = compact_size.value as usize;

        if bytes.len() < size_len + script_len {
            return Err(BitcoinError::InsufficientBytes);
        }

        let script_bytes = bytes[size_len..size_len + script_len].to_vec();
        Ok((Script::new(script_bytes), size_len + script_len))
    }
}

impl Deref for Script {
    type Target = Vec<u8>;
    fn deref(&self) -> &Self::Target {
        &self.bytes
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct TransactionInput {
    pub previous_output: OutPoint,
    pub script_sig: Script,
    pub sequence: u32,
}

impl TransactionInput {
    pub fn new(previous_output: OutPoint, script_sig: Script, sequence: u32) -> Self {
        Self {
            previous_output,
            script_sig,
            sequence,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.previous_output.to_bytes());
        bytes.extend_from_slice(&self.script_sig.to_bytes());
        bytes.extend_from_slice(&self.sequence.to_le_bytes());
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<(Self, usize), BitcoinError> {
        let (outpoint, outpoint_size) = OutPoint::from_bytes(bytes)?;

        if bytes.len() <= outpoint_size {
            return Err(BitcoinError::InsufficientBytes);
        }

        let (script_sig, script_size) = Script::from_bytes(&bytes[outpoint_size..])?;

        let sequence_offset = outpoint_size + script_size;
        if bytes.len() < sequence_offset + 4 {
            return Err(BitcoinError::InsufficientBytes);
        }

        let mut sequence_bytes = [0u8; 4];
        sequence_bytes.copy_from_slice(&bytes[sequence_offset..sequence_offset + 4]);
        let sequence = u32::from_le_bytes(sequence_bytes);

        Ok((
            TransactionInput::new(outpoint, script_sig, sequence),
            sequence_offset + 4,
        ))
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct BitcoinTransaction {
    pub version: u32,
    pub inputs: Vec<TransactionInput>,
    pub lock_time: u32,
}

impl BitcoinTransaction {
    pub fn new(version: u32, inputs: Vec<TransactionInput>, lock_time: u32) -> Self {
        Self {
            version,
            inputs,
            lock_time,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Version (4 bytes, little-endian)
        bytes.extend_from_slice(&self.version.to_le_bytes());

        // Number of inputs (CompactSize)
        let input_count = CompactSize::new(self.inputs.len() as u64);
        bytes.extend_from_slice(&input_count.to_bytes());

        // Each input
        for input in &self.inputs {
            bytes.extend_from_slice(&input.to_bytes());
        }

        // Lock time (4 bytes, little-endian)
        bytes.extend_from_slice(&self.lock_time.to_le_bytes());

        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<(Self, usize), BitcoinError> {
        if bytes.len() < 4 {
            return Err(BitcoinError::InsufficientBytes);
        }

        // Read version (4 bytes)
        let mut version_bytes = [0u8; 4];
        version_bytes.copy_from_slice(&bytes[0..4]);
        let version = u32::from_le_bytes(version_bytes);
        let mut offset = 4;

        // Read input count
        let (input_count, input_count_size) = CompactSize::from_bytes(&bytes[offset..])?;
        offset += input_count_size;

        // Read each input
        let mut inputs = Vec::new();
        for _ in 0..input_count.value {
            if offset >= bytes.len() {
                return Err(BitcoinError::InsufficientBytes);
            }

            let (input, input_size) = TransactionInput::from_bytes(&bytes[offset..])?;
            inputs.push(input);
            offset += input_size;
        }

        // Read lock time (4 bytes)
        if bytes.len() < offset + 4 {
            return Err(BitcoinError::InsufficientBytes);
        }
        let mut lock_time_bytes = [0u8; 4];
        lock_time_bytes.copy_from_slice(&bytes[offset..offset + 4]);
        let lock_time = u32::from_le_bytes(lock_time_bytes);
        offset += 4;

        Ok((BitcoinTransaction::new(version, inputs, lock_time), offset))
    }
}

impl fmt::Display for BitcoinTransaction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Transaction Version: {}\n", self.version)?;
        write!(f, "Input Count: {}\n", self.inputs.len())?;

        for (i, input) in self.inputs.iter().enumerate() {
            write!(f, "Input {}:\n", i)?;
            write!(
                f,
                "  Previous Output: {}\n",
                hex::encode(&input.previous_output.txid.0)
            )?;
            write!(
                f,
                "  Previous Output Vout: {}\n",
                input.previous_output.vout
            )?;
            write!(
                f,
                "  ScriptSig Length: {} bytes\n",
                input.script_sig.bytes.len()
            )?;
            write!(f, "  ScriptSig: {}\n", hex::encode(&input.script_sig.bytes))?;
            write!(f, "  Sequence: {}\n", input.sequence)?;
        }

        write!(f, "Lock Time: {}", self.lock_time)
    }
}
