// AUTHORS: Daniel Huici and Ricardo J. Rodríguez
// Copyright (c) 2025 - 2026
// GPLv3 License
// reverseame@unizar.es

use crate::controllers::apotheosis::Apotheosis;
use crate::datalayer::algorithms::DistanceAlgorithm;
use crate::datalayer::record::ApotheosisRecord;
use gexf::{Edge, EdgeType, Gexf, Node as GefxNode};
use std::fs;
use std::path::{Path, PathBuf};

/// Exports the HNSW model to GEXF files for Gephi visualization.
/// Creates one file per layer with pattern `<path>_layer<N>.gexf`.
/// Nodes include all metadata attributes, edges represent HNSW connections.
///
/// # Parameters
/// * `model` - The Apotheosis model to export
/// * `path` - Base filename for output (e.g., "model" creates "model_layer0.gexf", "model_layer1.gexf", etc.)
pub fn draw<R, D, const M: usize, const M0: usize, const EF: usize, const HEURISTIC: bool, P>(
    model: &Apotheosis<R, D, M, M0, EF, HEURISTIC>,
    path: P,
) where
    R: ApotheosisRecord,
    D: DistanceAlgorithm<R::MetricId> + Default,
    P: AsRef<Path>,
{
    let base_path = path.as_ref();

    let layer_gexfs = model.draw_model();

    // Enrich each layer with feature data and save
    for (layer_idx, (nodes, edges)) in layer_gexfs.iter().enumerate() {
        let mut gexf = Gexf::new(EdgeType::Undirected);
        for node in nodes {
            let mut gexf_node = GefxNode::new(node.to_string());
            let record = model
                .record(*node)
                .expect("node feature index out of range");
            for (key, val) in record.get_attributes() {
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
                .with_weight(*distance),
            );
        }

        let _ = save_gexf(base_path, layer_idx, &gexf);
    }
}

fn save_gexf(base_path: &Path, layer_idx: usize, gexf: &Gexf) -> std::io::Result<()> {
    let mut file_path = PathBuf::from(base_path);

    if let Some(stem) = file_path.file_stem().and_then(|s| s.to_str()) {
        let parent = file_path.parent().unwrap_or_else(|| Path::new(""));
        let filename = format!("{}_layer{}.gexf", stem, layer_idx);
        file_path = parent.join(filename);
    } else {
        let filename = format!("layer{}.gexf", layer_idx);
        file_path = PathBuf::from(filename);
    }

    fs::write(file_path, gexf.to_string().unwrap())?;
    Ok(())
}
