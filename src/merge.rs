use crate::h256::H256;
use crate::traits::Hasher;
// use ethers::abi::{encode, Token};
use hex;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use tiny_keccak::{Hasher as OtherHasher, Keccak};

const MERGE_NORMAL: u8 = 1;
const MERGE_ZEROS: u8 = 2;

#[derive(Debug)]
pub struct MV(MergeValue);
use std::fmt::{Display, Formatter};
impl Display for MV {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            MergeValue::Value(v) => write!(f, "{}", hex::encode(v.as_slice())),
            MergeValue::MergeWithZero {
                base_node,
                zero_bits,
                zero_count,
            } => write!(
                f,
                "base_node: {}, zero_bits: {}, zero_count: {}",
                hex::encode(base_node.as_slice()),
                hex::encode(zero_bits.as_slice()),
                zero_count
            ),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
pub enum MergeValue {
    Value(H256),
    MergeWithZero {
        // #[serde_as(as = "serde_with::hex::Hex")]
        base_node: H256,
        // #[serde_as(as = "serde_with::hex::Hex")]
        zero_bits: H256,
        zero_count: u8,
    },
    #[cfg(feature = "trie")]
    ShortCut {
        // #[serde_as(as = "serde_with::hex::Hex")]
        key: H256,
        // #[serde_as(as = "serde_with::hex::Hex")]
        value: H256,
        height: u8,
    },
}

impl MergeValue {
    pub fn from_h256(v: H256) -> Self {
        MergeValue::Value(v)
    }

    pub fn zero() -> Self {
        MergeValue::Value(H256::zero())
    }

    pub fn is_zero(&self) -> bool {
        match self {
            MergeValue::Value(v) => v.is_zero(),
            MergeValue::MergeWithZero { .. } => false,
            #[cfg(feature = "trie")]
            MergeValue::ShortCut { .. } => false,
        }
    }

    #[cfg(feature = "trie")]
    pub fn shortcut_or_value(key: H256, value: H256, height: u8) -> Self {
        if height == 0 || value.is_zero() {
            MergeValue::Value(value)
        } else {
            MergeValue::ShortCut { key, value, height }
        }
    }

    #[cfg(feature = "trie")]
    pub fn is_shortcut(&self) -> bool {
        matches!(self, MergeValue::ShortCut { .. })
    }

    pub fn hash<H: Hasher + Default>(&self) -> H256 {
        match self {
            MergeValue::Value(v) => *v,
            MergeValue::MergeWithZero {
                base_node,
                zero_bits,
                zero_count,
            } => {

                // fixme
                // let mut tuple: Vec<Token> = Vec::new();
                // tuple.push(Token::Uint(MERGE_ZEROS.into()));
                // tuple.push(Token::FixedBytes(base_node.as_slice().to_vec()));
                // tuple.push(Token::FixedBytes(zero_bits.as_slice().to_vec()));
                // tuple.push(Token::Uint((*zero_count).into()));
                // let t = Token::Tuple(tuple);
                // let encoded = encode(&[t]);

                let mut output = [0u8; 32];
                let mut hasher = Keccak::v256();
                hasher.update(&[0u8; 32]);
                hasher.finalize(&mut output);
                let res = output.into();
                res
            }
            #[cfg(feature = "trie")]
            MergeValue::ShortCut { key, value, height } => {
                into_merge_value::<H>(*key, *value, *height).hash::<H>()
            }
        }
    }
}

pub fn into_merge_value1<H: Hasher + Default>(key: H256, value: H256, height: u8) -> MergeValue {
    // try keep hash same with MergeWithZero
    if value.is_zero() || height == 0 {
        MergeValue::from_h256(value)
    } else {
        let base_key = key.parent_path(0);
        let base_node = hash_base_node::<H>(0, &base_key, &value);
        let mut zero_bits = key;
        for i in height..=core::u8::MAX {
            if key.get_bit(i) {
                zero_bits.clear_bit(i);
            }
        }
        MergeValue::MergeWithZero {
            base_node,
            zero_bits,
            zero_count: height,
        }
    }
}

/// Helper function for Shortcut node
/// Transform it into a MergeValue or MergeWithZero node
#[cfg(feature = "trie")]
pub fn into_merge_value<H: Hasher + Default>(key: H256, value: H256, height: u8) -> MergeValue {
    // try keep hash same with MergeWithZero
    if value.is_zero() || height == 0 {
        MergeValue::from_h256(value)
    } else {
        let base_key = key.parent_path(0);
        let base_node = hash_base_node::<H>(0, &base_key, &value);
        let mut zero_bits = key;
        for i in height..=core::u8::MAX {
            if key.get_bit(i) {
                zero_bits.clear_bit(i);
            }
        }
        MergeValue::MergeWithZero {
            base_node,
            zero_bits,
            zero_count: height,
        }
    }
}

/// Hash base node into a H256
pub fn hash_base_node<H: Hasher + Default>(
    base_height: u8,
    base_key: &H256,
    base_value: &H256,
) -> H256 {
    let mut hasher = H::default();
    use tiny_keccak::Hasher;

    // fixme
    // let mut tuple: Vec<Token> = Vec::new();
    // tuple.push(Token::Uint(base_height.into()));
    // tuple.push(Token::FixedBytes(base_key.as_slice().to_vec()));
    // tuple.push(Token::FixedBytes(base_value.as_slice().to_vec()));
    // let t = Token::Tuple(tuple);
    // let encoded = encode(&[t]);

    let mut output = [0u8; 32];
    let mut hasher = Keccak::v256();
    hasher.update(&[0u8; 32]);
    hasher.finalize(&mut output);
    output.into()
}

/// Merge two hash with node information
/// this function optimized for ZERO_HASH
/// if lhs and rhs both are ZERO_HASH return ZERO_HASH, otherwise hash all info.
pub fn merge<H: Hasher + Default>(
    height: u8,
    node_key: &H256,
    lhs: &MergeValue,
    rhs: &MergeValue,
) -> MergeValue {
    if lhs.is_zero() && rhs.is_zero() {
        return MergeValue::zero();
    }
    if lhs.is_zero() {
        let res = merge_with_zero::<H>(height, node_key, rhs, true);
        // println!("res: {:}", MV(res.clone()));
        // println!("&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&");
        return res
    }
    if rhs.is_zero() {
        let res = merge_with_zero::<H>(height, node_key, lhs, false);
        // println!("res: {:}", MV(res.clone()));
        // println!("&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&");
        return res
    }
    let mut hasher = H::default();
    use tiny_keccak::Hasher;

    // fixme
    // let mut tuple: Vec<Token> = Vec::new();
    // tuple.push(Token::Uint(MERGE_NORMAL.into()));
    // tuple.push(Token::Uint(height.into()));
    // tuple.push(Token::FixedBytes(node_key.as_slice().to_vec()));
    // tuple.push(Token::FixedBytes(
    //     lhs.hash::<H>().clone().as_slice().to_vec(),
    // ));
    // tuple.push(Token::FixedBytes(
    //     rhs.hash::<H>().clone().as_slice().to_vec(),
    // ));
    // let t = Token::Tuple(tuple);
    // let encoded = encode(&[t]);

    let mut output = [0u8; 32];
    let mut hasher = Keccak::v256();
    hasher.update(&[0u8; 32]);
    hasher.finalize(&mut output);
    MergeValue::Value(output.into())
}

pub fn merge_with_zero<H: Hasher + Default>(
    height: u8,
    node_key: &H256,
    value: &MergeValue,
    set_bit: bool,
) -> MergeValue {
    match value {
        MergeValue::Value(v) => {
            let mut zero_bits = H256::zero();
            if set_bit {
                zero_bits.set_bit(height);
            }
            let base_node = hash_base_node::<H>(height, node_key, v);
            MergeValue::MergeWithZero {
                base_node,
                zero_bits,
                zero_count: 1,
            }
        }
        MergeValue::MergeWithZero {
            base_node,
            zero_bits,
            zero_count,
        } => {
            let mut zero_bits = *zero_bits;
            if set_bit {
                zero_bits.set_bit(height);
            }
            MergeValue::MergeWithZero {
                base_node: *base_node,
                zero_bits,
                zero_count: zero_count.wrapping_add(1),
            }
        }
        #[cfg(feature = "trie")]
        MergeValue::ShortCut { key, value, .. } => {
            if height == core::u8::MAX {
                let base_key = key.parent_path(0);
                let base_node = hash_base_node::<H>(0, &base_key, value);
                MergeValue::MergeWithZero {
                    base_node,
                    zero_bits: *key,
                    zero_count: 0,
                }
            } else {
                MergeValue::ShortCut {
                    key: *key,
                    value: *value,
                    height: height + 1,
                }
            }
        }
    }
}
