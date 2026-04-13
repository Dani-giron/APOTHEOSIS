// AUTHORS: Daniel Huici and Ricardo J. Rodríguez
// Copyright (c) 2025 - 2026
// GPLv3 License
// reverseame@unizar.es

use crate::controllers::hnsw::Hnsw;
use crate::controllers::radix_tree::RadixNode;
use crate::datalayer::algorithms::DistanceAlgorithm;
use crate::datalayer::record::ApotheosisRecord;
use gexf::{Edge, EdgeType, Gexf, Node as GefxNode};
use std::fs::{self};
use std::path::{Path, PathBuf};

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(bound(
    serialize = "R: ApotheosisRecord + serde::Serialize, D: DistanceAlgorithm<R::MetricId> + serde::Serialize, R::MetricId: serde::Serialize",
    deserialize = "R: ApotheosisRecord + serde::Deserialize<'de>, D: DistanceAlgorithm<R::MetricId> + serde::Deserialize<'de>, R::MetricId: serde::Deserialize<'de>"
))]
pub struct Apotheosis<R, D, const M: usize = 16, const M0: usize = 32, const EF: usize = 400>
where
    R: ApotheosisRecord,
    D: DistanceAlgorithm<R::MetricId> + Default,
{
    pub hnsw: Hnsw<D, R::MetricId, M, M0, EF>,
    pub radix: RadixNode<u8, Option<usize>>,
    pub records: Vec<R>,
}

impl<R, D, const M: usize, const M0: usize, const EF: usize> Apotheosis<R, D, M, M0, EF>
where
    R: ApotheosisRecord,
    D: DistanceAlgorithm<R::MetricId> + Default,
{
    pub fn new() -> Self {
        Self {
            hnsw: Hnsw::new(),
            radix: RadixNode::<u8, Option<usize>>::new(vec![], None),
            records: vec![],
        }
    }

    /// Inserts an ApotheosisRecord into the Apotheosis Model (radix tree + HNSW + vector metadata).
    /// The record's `search_id` is stored in HNSW for similarity search, the `radix_key` is mapped
    /// to the record's index, and the record itself is stored for later retrieval.
    ///
    /// # Parameters
    /// * `item` - The ApotheosisRecord to insert
    ///
    /// # Returns
    /// * `true` if the record was successfully inserted
    /// * `false` if the record's key already exists in the model
    pub fn insert(&mut self, item: R) -> bool {
        let hnsw_node_index = self.hnsw.insert(item.search_id());

        // Only insert into the Radix fast-path if the metric natively maps to a string key
        if let Some(key_to_insert) = item.search_id().to_radix_key() {
            if self.radix.find(&key_to_insert).is_some() {
                println!("Key already exists in radix tree: {:?}", key_to_insert);
                return false;
            }
            self.radix.insert(key_to_insert, Some(hnsw_node_index));
        }

        self.records.push(item);
        true
    }

    /// Performs an approximate k-NN search. If the `MetricId` natively maps to a Radix Tree key 
    /// (e.g. TLSH string), it jumps directly to the HNSW node to retrieve neighbors, bypassing full traversal.
    /// Otherwise, it performs a standard HNSW k-NN search using the `query`.
    ///
    /// # Parameters
    /// * `query` - The native HNSW numerical or hash representation
    /// * `k` - Number of nearest neighbors to return
    ///
    /// # Returns
    /// * `Vec<(u32, &R)>` - List of tuples containing:
    ///   - `u32`: Distance/score to the query
    ///   - `&R`: Reference to the actual retrieved Record item
    pub fn search(
        &self,
        query: &R::MetricId,
        k: usize,
        ef_search: Option<usize>,
    ) -> Vec<(u32, &R)> {
        let ef_search = ef_search.unwrap_or(24);

        let hnsw_results: Vec<(u32, usize, &R::MetricId)> = if let Some(key) = query.to_radix_key() {
            if let Some(radix_node) = self.radix.find(&key) {
                if let Some(Some(node_index)) = radix_node.data {
                    // Exact match found! Jump directly to neighbors in HNSW
                    self.hnsw.get_neighbors_node(node_index)
                } else {
                    self.hnsw.knn_search(query, k, ef_search)
                }
            } else {
                self.hnsw.knn_search(query, k, ef_search)
            }
        } else {
            self.hnsw.knn_search(query, k, ef_search)
        };

        hnsw_results
            .into_iter()
            .map(|(distance, index, _id)| (distance, &self.records[index]))
            .collect()
    }

    /// Dumps the Apotheosis model to a binary file using Bincode.
    pub fn dump<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>>
    where
        Self: serde::Serialize,
    {
        let encoded = bincode::serialize(self)?;
        fs::write(path, encoded)?;
        Ok(())
    }

    /// Loads an Apotheosis model from a binary file using Bincode.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>>
    where
        Self: serde::de::DeserializeOwned,
    {
        let data = fs::read(path)?;
        let decoded = bincode::deserialize(&data)?;
        Ok(decoded)
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
                for (key, val) in self.records[*node].get_attributes() {
                    gexf_node = gexf_node.with_attr(key, val);
                }
                let _ = gexf.add_node(gexf_node);
            }

            for (source_id, target_id, distance) in edges {
                let _ = gexf.add_edge(
                    Edge::new(
                        format!("e_{}_{}", source_id, target_id),
                        source_id.clone(),
                        target_id.clone(),
                    )
                    .with_weight(distance.clone()),
                );
            }

            let _ = self.save_gexf(base_path, layer_idx, &gexf);
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

        let fixed_xml = if !self.records.is_empty() {
            self.add_attribute_schema(gexf.to_string().unwrap(), self.records[0].get_attributes())
        } else {
            gexf.to_string().unwrap()
        };

        fs::write(file_path, fixed_xml)?;
        Ok(())
    }
}
