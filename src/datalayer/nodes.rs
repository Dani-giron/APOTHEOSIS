use std::hash::{Hash, Hasher};

#[derive(Debug, Clone)]
pub struct Node<const N: usize> {
    pub next_node: usize, // Pointer to this node but in the next layer
    pub feature_index: usize,
    pub neighbors: [usize; N], // TODO: Save the distances aswell?
    pub neighbor_count: usize,  
}

impl<const N: usize> Node<N> {
    pub fn active_neighbors(&self) -> &[usize] {
        &self.neighbors[..self.neighbor_count]
    }
}

impl<const N: usize> Hash for Node<N> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.next_node.hash(state);
        self.feature_index.hash(state);
        self.active_neighbors().hash(state);
    }
}

impl<const N: usize> PartialEq for Node<N> {
    fn eq(&self, other: &Self) -> bool {
        self.next_node == other.next_node
            && self.feature_index == other.feature_index // Should be enough with this I guess
            && self.active_neighbors() == other.active_neighbors()
    }
}

impl<const N: usize> Eq for Node<N> {}