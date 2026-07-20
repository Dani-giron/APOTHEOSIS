// AUTHORS: Daniel Huici and Ricardo J. Rodríguez
// Copyright (c) 2025 - 2026
// GPLv3 License
// reverseame@unizar.es

use crate::controllers::hnsw::Hnsw;
use crate::controllers::radix_tree::RadixNode;
use crate::datalayer::algorithms::DistanceAlgorithm;
use crate::datalayer::record::{ApotheosisRecord, RadixKeyMapping};
use gexf::{Edge, EdgeType, Gexf, Node as GefxNode};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// FNV-1a hash of a string, used to encode the distance type identity in the
/// model header. Chosen over `std` hashers because it is fully specified and
/// stable across Rust versions, which a persistence format requires.
const fn fnv1a(s: &str) -> u32 {
    let bytes = s.as_bytes();
    let mut hash: u32 = 0x811c_9dc5;
    let mut i = 0;
    while i < bytes.len() {
        hash ^= bytes[i] as u32;
        hash = hash.wrapping_mul(0x0100_0193);
        i += 1;
    }
    hash
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(bound(
    serialize = "R: ApotheosisRecord + serde::Serialize, D: DistanceAlgorithm<R::MetricId> + serde::Serialize, R::MetricId: serde::Serialize",
    deserialize = "R: ApotheosisRecord + serde::Deserialize<'de>, D: DistanceAlgorithm<R::MetricId> + serde::Deserialize<'de>, R::MetricId: serde::Deserialize<'de>"
))]
pub struct Apotheosis<
    R,
    D,
    const M: usize = 16,
    const M0: usize = 32,
    const EF: usize = 400,
    const HEURISTIC: bool = false,
> where
    R: ApotheosisRecord,
    D: DistanceAlgorithm<R::MetricId> + Default,
{
    pub hnsw: Hnsw<D, R::MetricId, M, M0, EF, HEURISTIC>,
    pub radix: RadixNode<u8, Option<usize>>,
    pub records: Vec<R>,
}

impl<R, D, const M: usize, const M0: usize, const EF: usize, const HEURISTIC: bool>
    Apotheosis<R, D, M, M0, EF, HEURISTIC>
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
        let radix_key = item.search_id().to_radix_key();

        if let Some(ref key) = radix_key {
            if self.radix.find(key).is_some() {
                println!("Key already exists in radix tree: {:?}", key);
                return false;
            }
        }

        let hnsw_node_index = self.hnsw.insert(item.search_id());
        if let Some(key) = radix_key {
            self.radix.insert(key, Some(hnsw_node_index));
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
        if self.records.is_empty() {
            return vec![];
        }

        let ef_search = ef_search.unwrap_or(24);

        let hnsw_results: Vec<(u32, usize, &R::MetricId)> = if let Some(key) = query.to_radix_key()
        {
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

        let mut hnsw_results = hnsw_results;
        hnsw_results.sort_by_key(|(d, _, _)| *d);
        hnsw_results
            .into_iter()
            .take(k)
            .map(|(distance, index, _id)| (distance, &self.records[index]))
            .collect()
    }

    pub fn dump<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>>
    where
        Self: serde::Serialize,
    {
        let mut file = File::create(path)?;
        // Write magic bytes: "APOT" (4 bytes)
        file.write_all(b"APOT")?;
        // Write M, M0, EF as u32 (12 bytes)
        file.write_all(&(M as u32).to_le_bytes())?;
        file.write_all(&(M0 as u32).to_le_bytes())?;
        file.write_all(&(EF as u32).to_le_bytes())?;
        file.write_all(&[HEURISTIC as u8])?;
        // Write the distance type identity as an FNV-1a hash (4 bytes)
        let distance_id = fnv1a(<D as DistanceAlgorithm<R::MetricId>>::TYPE_NAME);
        file.write_all(&distance_id.to_le_bytes())?;

        // Then write the model data
        bincode::serialize_into(file, self)?;
        Ok(())
    }

    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>>
    where
        Self: serde::de::DeserializeOwned,
    {
        let mut file = File::open(path)?;

        // Read and verify header (21 bytes)
        let mut header = [0u8; 21];
        file.read_exact(&mut header)?;

        if &header[0..4] != b"APOT" {
            return Err("Invalid Apotheosis model file (missing magic bytes)".into());
        }

        let m = u32::from_le_bytes(header[4..8].try_into()?);
        let m0 = u32::from_le_bytes(header[8..12].try_into()?);
        let ef = u32::from_le_bytes(header[12..16].try_into()?);
        let heuristic = header[16] != 0;

        if m != M as u32 || m0 != M0 as u32 || ef != EF as u32 || heuristic != HEURISTIC {
            return Err(format!(
                "Model parameter mismatch. File has M={}, M0={}, EF={}, HEURISTIC={} but code expects M={}, M0={}, EF={}, HEURISTIC={}",
                m, m0, ef, heuristic, M, M0, EF, HEURISTIC
            ).into());
        }

        // The distance type is not part of the serialized body (bincode does
        // not depend on D), so it must be checked explicitly: loading a model
        // built with one distance into a type using another returns wrong
        // results with no error otherwise.
        let file_distance_id = u32::from_le_bytes(header[17..21].try_into()?);
        let expected_distance_id = fnv1a(<D as DistanceAlgorithm<R::MetricId>>::TYPE_NAME);
        if file_distance_id != expected_distance_id {
            return Err(format!(
                "Distance type mismatch. File was built with a different distance than the loading type (expected {}). Load the model with the distance it was created with",
                <D as DistanceAlgorithm<R::MetricId>>::TYPE_NAME
            )
            .into());
        }

        let decoded = bincode::deserialize_from(file)?;
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
