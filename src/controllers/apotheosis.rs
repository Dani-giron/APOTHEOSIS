// AUTHORS: Daniel Huici and Ricardo J. Rodríguez
// Copyright (c) 2025
// GPLv3 License
// reverseame@unizar.es


use radix_tree::{Node, Radix};
use crate::controllers::hnsw::Hnsw;
use crate::datalayer::algorithms::DistanceAlgorithm;
use crate::datalayer::features::FeatureType;

pub struct Apotheosis<F, D, const M: usize, const M0: usize, const EF: usize = 400> 
where 
    F: FeatureType,
    F::IdType: Clone, 
    D: DistanceAlgorithm<F::IdType>,
{
    pub hnsw: Hnsw<D, F::IdType, M, M0, EF>,
    pub radix: Node<char, Option<usize>>,
}

pub struct ApotheosisResult<ID> {
    pub exact_match: bool,
    pub distance: u32,
    pub feature: ID,
}

impl<F, D, const M: usize, const M0: usize, const EF: usize> Apotheosis<F, D, M, M0, EF> 
where 
    F: FeatureType,
    F::IdType: Clone,  
    D: DistanceAlgorithm<F::IdType>
{
    pub fn new() -> Self {
        Self {
            hnsw: Hnsw::new(),
            radix: Node::<char, Option<usize>>::new("", None),
        }
    }

    /// Inserts a feature into Apotheosis Model (radix tree + HNSW).
    /// 
    /// # Parameters
    /// * `feature` - The feature to insert
    /// 
    /// # Returns
    /// * `true` if the feature was successfully inserted
    /// * `false` if the feature's key already exists in the model
    pub fn insert(&mut self, feature: F) -> bool {
        let key_to_find = feature.get_radix_key().clone();
        let key_to_insert = feature.get_radix_key().clone();

        if self.radix.find(key_to_find.clone()).is_some() {
            println!("Key already exists in radix tree: {:?}", key_to_find);
            return false;
        }
        
        let hnsw_node_index = self.hnsw.insert(feature.get_id().clone());
        self.radix.insert(key_to_insert, Some(hnsw_node_index));
        
        true
    }

    /// Performs a search operation in the Apotheosis Model.
    /// - First checks the radix tree for the key. If found, retrieves neighbors from HNSW.
    /// - If not found, performs an HNSW search using the provided feature.
    /// 
    /// # Parameters
    /// * `key` - The key to search for in the radix tree
    /// * `k` - Number of neighbors to return
    /// 
    /// # Returns
    /// * `Vec<(u32, &F)>` - List of tuples with distance and feature reference
    pub fn search(&self, key: String, k: usize) -> Vec<(u32, &F::IdType)> {
        let temp_feature = F::create(key);

        if let Some(radix_node) = self.radix.find(temp_feature.get_radix_key().clone()) {
            if let Some(Some(node_index)) = radix_node.data {
                return self.hnsw.get_zero_layer_node(node_index);
            }
        }
        
        let hnsw_results: Vec<(u32, &F::IdType)> = self.hnsw.knn_search(temp_feature.get_id(), 24, k);
        hnsw_results
    }

}
