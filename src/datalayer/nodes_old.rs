use std::hash::{Hash, Hasher};

pub struct Node {
    pub next_node: usize,
    pub feature_index: usize,
    pub neighbors: Vec<(usize, u32)>,
    //pub neighbors: PriorityQueue<usize, u32>,
}

impl Hash for Node {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.next_node.hash(state);
        self.feature_index.hash(state);  // &F implements Hash if F does
        self.neighbors.hash(state);
    }
}

impl  PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.next_node == other.next_node
            && self.feature_index == other.feature_index  // Comparing &F references
            && self.neighbors == other.neighbors
    }
}

impl Eq for Node {}
