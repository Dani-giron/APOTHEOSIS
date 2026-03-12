extern crate alloc;

use alloc::borrow::ToOwned;
use alloc::vec;
use alloc::vec::Vec;
use core::mem;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RadixNode<K, V> {
    pub path: Vec<K>,    // The path (subset of the key) associated with this node
    pub data: Option<V>, // The data stored at this node, if any
    pub indices: Vec<K>, // The first element of each child's path (indexes of children nodes)
    pub nodes: Vec<RadixNode<K, V>>, // The children nodes
}

impl<K, V> RadixNode<K, V>
where
    K: Copy + PartialEq + PartialOrd,
    V: Clone,
{
    /// Creates a new RadixNode with the given path and data.
    pub fn new(path: Vec<K>, data: Option<V>) -> Self {
        RadixNode {
            data,
            path,
            nodes: Vec::new(),
            indices: Vec::new(),
        }
    }

    /// Inserts a value into the Radix Tree at the specified path.
    ///
    /// If the path already exists, the data is updated. If the path diverges from
    /// existing paths, the tree is split accordingly.
    ///
    /// # Parameters
    /// * `path` - The key/path where the data should be inserted.
    /// * `data` - The value to store at the given path.
    ///
    /// # Returns
    /// * `&mut Self` - A mutable reference to the node where the data was inserted.
    pub fn insert<P: AsRef<[K]>>(&mut self, path: P, data: V) -> &mut Self {
        let path = path.as_ref();
        let new_key_len = path.len();
        let node_path_len = self.path.len();

        // Empty key
        if new_key_len == 0 {
            self.data = Some(data);
            return self;
        }

        // The radix tree is empty
        if node_path_len == 0 && self.indices.is_empty() {
            self.data = Some(data);
            self.path = path.to_owned();
            return self;
        }

        // Find the common prefix length
        let prefix_len = path
            .iter()
            .zip(&self.path)
            .take_while(|(a, b)| a == b)
            .count();

        // Split the current node if the new key diverges early
        if prefix_len < node_path_len {
            let child = RadixNode {
                data: self.data.take(),
                path: self.path.split_off(prefix_len),
                nodes: mem::take(&mut self.nodes),
                indices: mem::take(&mut self.indices),
            };
            self.indices = vec![child.path[0]];
            self.nodes = vec![child];
        }

        // The new key matches the current node's path
        if prefix_len == new_key_len {
            self.data = Some(data);
            return self;
        }

        // Else, delegate to a child, or create a new leaf
        let branch_key = path[prefix_len];

        // Check if a child already handles this path
        if let Some(child_idx) = self.indices.iter().position(|&x| x == branch_key) {
            return self.nodes[child_idx].insert(&path[prefix_len..], data);
        }

        // In case we didn't find a matching child, we create a new leaf node
        self.indices.push(branch_key);
        self.nodes.push(RadixNode {
            data: Some(data),
            nodes: Vec::new(),
            indices: Vec::new(),
            path: path[prefix_len..].to_owned(),
        });

        // Return a mutable reference to the newly pushed leaf
        self.nodes.last_mut().unwrap()
    }

    /// Finds a node in the Radix Tree that matches the specified path.
    ///
    /// # Parameters
    /// * `path` - The key/path to search for.
    ///
    /// # Returns
    /// * `Option<&Self>` - A reference to the node if found, or `None` otherwise.
    pub fn find<P: AsRef<[K]>>(&self, path: P) -> Option<&Self> {
        let path = path.as_ref();
        let query_len = path.len();
        let node_path_len = self.path.len();

        // The search path is shorter than the node's path. It cannot be here
        if query_len < node_path_len {
            return None;
        }

        // Find where the query diverges from the key
        let prefix_len = path
            .iter()
            .zip(&self.path)
            .take_while(|(a, b)| a == b)
            .count();

        // If the match didn't cover the entire node's path, it diverges here
        if prefix_len < node_path_len {
            return None;
        }

        // Query and node path both lengths matches. FOUND!
        if query_len == node_path_len {
            return Some(self);
        }

        // Query is longer, so we need to look into the children
        let branch_char = path[prefix_len];
        if let Some(child_idx) = self.indices.iter().position(|&x| x == branch_char) {
            return self.nodes[child_idx].find(&path[prefix_len..]);
        }

        // No matching child found
        None
    }
}
