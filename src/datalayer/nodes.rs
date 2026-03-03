use std::hash::{Hash, Hasher};

#[derive(Debug, Clone)]
pub struct HnswNode<const N: usize> {
    pub next_node: u32,     // Pointer to this node but in the next layer
    pub feature_index: u32, // Pointer to the data associated to this node
    pub neighbors: [u32; N],
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
