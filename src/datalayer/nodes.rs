use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::hash::{Hash, Hasher};

/// A node in the HNSW data structure
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HnswNode<const N: usize> {
    pub next_node: u32,     // Pointer to this node but in the next layer
    pub feature_index: u32, // Pointer to the data associated to this node
    #[serde(with = "serde_array")]
    pub neighbors: [u32; N],
    #[serde(with = "serde_array")]
    pub neighbor_distances: [u32; N],
    pub neighbor_count: u16,
}

impl<const N: usize> HnswNode<N> {
    pub fn new_empty(feature_index: u32, next_node: u32) -> Self {
        Self {
            next_node,
            feature_index,
            neighbors: [u32::MAX; N],
            neighbor_distances: [u32::MAX; N],
            neighbor_count: 0,
        }
    }

    #[inline]
    pub fn active_neighbors(&self) -> &[u32] {
        &self.neighbors[..self.neighbor_count as usize]
    }

    #[inline]
    pub fn active_distances(&self) -> &[u32] {
        &self.neighbor_distances[..self.neighbor_count as usize]
    }
}

impl<const N: usize> Hash for HnswNode<N> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.next_node.hash(state);
        self.feature_index.hash(state);
        self.neighbor_count.hash(state);
    }
}

impl<const N: usize> PartialEq for HnswNode<N> {
    fn eq(&self, other: &Self) -> bool {
        self.next_node == other.next_node
            && self.feature_index == other.feature_index // Should be enough with this I guess
            && self.neighbor_count == other.neighbor_count
    }
}

impl<const N: usize> Eq for HnswNode<N> {}

// Helper module for serializing and deserializing const-generic arrays
// Serde does not support const-generic arrays directly so we need to do this garbage
mod serde_array {
    use super::*;

    pub fn serialize<S, T, const N: usize>(array: &[T; N], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: Serialize,
    {
        array.as_slice().serialize(serializer)
    }

    pub fn deserialize<'de, D, T, const N: usize>(deserializer: D) -> Result<[T; N], D::Error>
    where
        D: Deserializer<'de>,
        T: Deserialize<'de> + Copy + Default,
    {
        let vec: Vec<T> = Vec::deserialize(deserializer)?;
        if vec.len() != N {
            return Err(serde::de::Error::custom(format!(
                "expected array of length {}, found {}",
                N,
                vec.len()
            )));
        }
        let mut array = [T::default(); N];
        array[..vec.len()].copy_from_slice(&vec);
        Ok(array)
    }
}
