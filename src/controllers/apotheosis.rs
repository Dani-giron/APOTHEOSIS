// AUTHORS: Daniel Huici and Ricardo J. Rodríguez
// Copyright (c) 2025
// GPLv3 License
// reverseame@unizar.es

use radix_tree::{Node as RadixNode, Radix};
use crate::datalayer::metadata::{ApotheosisMetadata};
use crate::{controllers::hnsw::Hnsw};
use crate::datalayer::algorithms::DistanceAlgorithm;
use crate::datalayer::features::FeatureType;
use gexf::{Edge, EdgeType, Gexf, Node as GefxNode};
use std::fs::{self};
use std::path::{Path, PathBuf};

pub struct Apotheosis<F, D, Meta = (), const M: usize = 16, const M0: usize = 32, const EF: usize = 400> 
where 
    F: FeatureType,
    F::IdType: Clone, 
    D: DistanceAlgorithm<F::IdType>,
    Meta: ApotheosisMetadata
{
    pub hnsw: Hnsw<D, F::IdType, M, M0, EF>,
    pub radix: RadixNode<char, Option<usize>>,
    pub metadata: Vec<Meta>,
}

impl<F, D, Meta, const M: usize, const M0: usize, const EF: usize> Apotheosis<F, D, Meta, M, M0, EF> 
where 
    F: FeatureType,
    F::IdType: Clone,  
    D: DistanceAlgorithm<F::IdType>,
    Meta: ApotheosisMetadata
{
    pub fn new() -> Self {
        Self {
            hnsw: Hnsw::new(),
            radix: RadixNode::<char, Option<usize>>::new("", None),
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
    pub fn search(&self, key: String, k: usize, ef_search: Option<usize>) -> Vec<(u32, &F::IdType, &Meta)> {
        let temp_feature = F::create( key);
        let ef_search = ef_search.unwrap_or(24); 

        let hnsw_results: Vec<(u32, usize, &F::IdType)> = 
            if let Some(radix_node) = self.radix.find(temp_feature.get_radix_key().clone()) {
            if let Some(Some(node_index)) = radix_node.data {
                self.hnsw.get_neighbors_node(node_index)
            } else {
                self.hnsw.knn_search(temp_feature.get_id(), k, ef_search)
            }
        } else {
            self.hnsw.knn_search(temp_feature.get_id(), k, ef_search)
        };
        
        hnsw_results
            .into_iter()
            .map(|(distance, index, id)| {
                (distance, id, &self.metadata[index])
            })
            .collect()
    }

    // Exports the HNSW model to GEXF files for Gephi visualization.
    /// Creates one file per layer with pattern `<path>_layer<N>.gexf`.
    /// Nodes include all metadata attributes, edges represent HNSW connections.
    ///
    /// # Parameters
    /// * `path` - Base filename for output (e.g., "model" creates "model_layer0.gexf", "model_layer1.gexf", etc.)

    pub fn draw<P: AsRef<Path>>(&self, path: P) {
        let base_path = path.as_ref();
        
        let layer_gexfs = self.hnsw.draw_model();
        
        // Enrich each layer with feature data and save
        for (layer_idx, (nodes, edges)) in layer_gexfs.iter().enumerate() {
            let mut gexf = Gexf::new(EdgeType::Undirected);
            for node in nodes {
                let mut gexf_node = GefxNode::new(node.to_string());
                for (key, val) in self.metadata[*node].get_attributes() {
                    gexf_node = gexf_node.with_attr(key, val);
                }
                let _ = gexf.add_node(gexf_node);
            }

            for (source_id, target_id, distance) in edges {
                let _ = gexf.add_edge(Edge::new(
                    format!("e_{}_{}", source_id, target_id),
                    source_id.clone(),
                    target_id.clone(),
                ).with_weight(distance.clone()));
            }

            self.save_gexf(base_path, layer_idx, &gexf);
        }
        
    }


    // Once draw is built, we need to add the attribute schema to the GEXF XML
    // Has to be done manually since gexf crate does not support it yet.
   fn add_attribute_schema(&self, xml: String, attributes: Vec<(String, String)>) -> String {
        if let Some(graph_start) = xml.find("<graph") {
            if let Some(graph_end_offset) = xml[graph_start..].find(">") {
                let insert_pos = graph_start + graph_end_offset + 1;
                
                let mut attrs = String::from("\n    <attributes class=\"node\">\n");
                for (attribute_key, _) in attributes {
                    attrs.push_str(&format!(
                        "      <attribute id=\"{}\" title=\"{}\" type=\"string\"/>\n",
                        attribute_key, attribute_key
                    ));
                }
                attrs.push_str("    </attributes>\n");
                
                let mut result = String::new();
                result.push_str(&xml[..insert_pos]);
                result.push_str(&attrs);
                result.push_str(&xml[insert_pos..]);
                
                return result;
            }
        }
        
        xml
    }
    

    fn save_gexf(&self, base_path: &Path, layer_idx: usize, gexf: &Gexf) -> std::io::Result<()> {
        let mut file_path = PathBuf::from(base_path);
        
        if let Some(stem) = file_path.file_stem().and_then(|s| s.to_str()) {
            let parent = file_path.parent().unwrap_or_else(|| Path::new(""));
            let filename = format!("{}_layer{}.gexf", stem, layer_idx);
            file_path = parent.join(filename);
        } else {
            let filename = format!("layer{}.gexf", layer_idx);
            file_path = PathBuf::from(filename);
        }

        let fixed_xml = self.add_attribute_schema(
            gexf.to_string().unwrap(),
            self.metadata[0].get_attributes(),
        );

        fs::write(file_path, fixed_xml)?;
        Ok(())
    }



}
