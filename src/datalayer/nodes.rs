use std::hash::{Hash, Hasher};

#[derive(Debug, Clone)]
pub struct Node<const N: usize> {
    pub next_node: usize, // Pointer to this node but in the next layer
    pub feature_index: usize,
    pub neighbors: [usize; N],
    pub neighbor_distances: [u32; N],
    pub neighbor_count: usize,

}

impl<const N: usize> Node<N> {
    pub fn new_empty(feature_index: usize, next_node: usize) -> Self {
        Self {
            next_node,
            feature_index,
            neighbors: [usize::MAX; N],
            neighbor_distances: [u32::MAX; N],
            neighbor_count: 0,
        }
    }

    #[inline]
    pub fn active_neighbors(&self) -> &[usize] {
        &self.neighbors[..self.neighbor_count]
    }

    pub fn active_distances(&self) -> &[u32] {
        &self.neighbor_distances[..self.neighbor_count]
    }
}

impl<const N: usize> Hash for Node<N> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.next_node.hash(state);
        self.feature_index.hash(state);
        self.neighbor_count.hash(state);
    }
}

impl<const N: usize> PartialEq for Node<N> {
    fn eq(&self, other: &Self) -> bool {
        self.next_node == other.next_node
            && self.feature_index == other.feature_index // Should be enough with this I guess
            && self.neighbor_count == other.neighbor_count
    }
}

impl<const N: usize> Eq for Node<N> {}