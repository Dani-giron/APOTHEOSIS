// AUTHORS: Daniel Huici and Ricardo J. Rodríguez
// Copyright (c) 2025
// GPLv3 License
// reverseame@unizar.es

use radix_tree::{Node, Radix};
use crate::controllers::hnsw::Hnsw;
use crate::datalayer::algorithms::DistanceAlgorithm;
use crate::datalayer::features::FeatureType;

pub struct Apotheosis<F, D, Meta = (), const M: usize = 16, const M0: usize = 32, const EF: usize = 400> 
where 
    F: FeatureType,
    F::IdType: Clone, 
    D: DistanceAlgorithm<F::IdType>,
{
    pub hnsw: Hnsw<D, F::IdType, M, M0, EF>,
    pub radix: Node<char, Option<usize>>,
    pub metadata: Vec<Meta>,
}

impl<F, D, Meta, const M: usize, const M0: usize, const EF: usize> Apotheosis<F, D, Meta, M, M0, EF> 
where 
    F: FeatureType,
    F::IdType: Clone,  
    D: DistanceAlgorithm<F::IdType>
{
    pub fn new() -> Self {
        Self {
            hnsw: Hnsw::new(),
            radix: Node::<char, Option<usize>>::new("", None),
            metadata: vec![],
        }
    }

    /// Inserts a feature with metadata into Apotheosis Model (radix tree + HNSW + metadata).
    /// The feature's ID is stored in HNSW for similarity search, the radix key is mapped
    /// to the feature's index, and the metadata is stored separately for later retrieval.
    /// 
    /// # Parameters
    /// * `feature` - The feature containing ID and radix key
    /// * `meta` - The metadata associated with this feature (use `()` for no metadata)
    /// 
    /// # Returns
    /// * `true` if the feature was successfully inserted
    /// * `false` if the feature's key already exists in the model
    pub fn insert(&mut self, feature: F, meta: Meta) -> bool {
        let key_to_find = feature.get_radix_key().clone();
        let key_to_insert = feature.get_radix_key().clone();

        if self.radix.find(key_to_find.clone()).is_some() {
            println!("Key already exists in radix tree: {:?}", key_to_find);
            return false;
        }
        
        let hnsw_node_index = self.hnsw.insert(feature.get_id().clone());
        self.metadata.push(meta);
        self.radix.insert(key_to_insert, Some(hnsw_node_index));
        
        true
    }

    /// Performs a search operation in the Apotheosis Model. First checks
    /// the radix tree for an exact key match. If found with data, retrieves
    /// neighbors from HNSW layer 0. If not found or no data attached,
    /// performs an approximate k-NN search using HNSW.
    /// 
    /// # Parameters
    /// * `key` - The key to search for in the radix tree
    /// * `k` - Number of nearest neighbors to return
    /// 
    /// # Returns
    /// * `Vec<(u32, &F::IdType, &Meta)>` - List of tuples containing:
    ///   - `u32`: Distance/score to the query
    ///   - `&F::IdType`: Reference to the feature's ID
    ///   - `&Meta`: Reference to the associated metadata (or `&()` if no metadata)
    pub fn search(&self, key: String, k: usize) -> Vec<(u32, &F::IdType, &Meta)> {
        let temp_feature = F::create(key);

        let hnsw_results: Vec<(u32, usize, &F::IdType)> = 
            if let Some(radix_node) = self.radix.find(temp_feature.get_radix_key().clone()) {
            if let Some(Some(node_index)) = radix_node.data {
                self.hnsw.get_neighbors_node(node_index)
            } else {
                self.hnsw.knn_search(temp_feature.get_id(), 24, k)
            }
        } else {
            self.hnsw.knn_search(temp_feature.get_id(), 24, k)
        };
        
        hnsw_results
            .into_iter()
            .map(|(distance, index, id)| {
                (distance, id, &self.metadata[index])
            })
            .collect()
    }
}
