use apotheosis2::datalayer::algorithms::{TLSHDistance};
use apotheosis2::datalayer::features::{FeatureType, HashFeature};
use apotheosis2::{controllers::hnsw::Hnsw};
use std::time::Instant;
use tlsh2::{TlshDefault};
use std::str::{FromStr};

use serde_json::Value;
use std::fs;


fn read_hashes_from_json<P: AsRef<std::path::Path>>(path: P) -> Vec<String> {
    let data = fs::read_to_string(path).expect("Failed to read JSON file");
    let v: Value = serde_json::from_str(&data).expect("Failed to parse JSON");
    let obj = v.as_object().expect("Expected JSON object at root");

    obj.values()
        .filter_map(|val| val.as_str().map(|s| s.to_string()))
        .collect()
}

fn create_tlsh_objects(hashes: &[String]) -> Vec<HashFeature> {
    hashes.iter()
        .filter_map(|h| TlshDefault::from_str(h).ok().map(HashFeature::new))
        .collect()
}


// cargo run --bin testing
pub fn main() {
    let hashes = read_hashes_from_json("tlsh_hashes.json");
    println!("Number of hashes: {:?}", hashes.len());

    let data = create_tlsh_objects(&hashes);

    // Initialize vectors before pushing
    let dataset: Vec<HashFeature> = data[..92000].to_vec();
    let dataset_copy: Vec<HashFeature> = dataset.clone();
    let queries: Vec<HashFeature> = data[92000..93000].to_vec();

    let mut model: Hnsw<TLSHDistance, HashFeature, 12, 24> = Hnsw::new(TLSHDistance, 12, 24, 400);
    let creation_start: Instant = Instant::now();

    for f in dataset_copy {
        model.insert(f);
    }

    let creation_time: std::time::Duration = creation_start.elapsed();

    let mut brute_results: Vec<(u32, &HashFeature)> = Vec::new();
    let mut apo_results: Vec<(u32, &HashFeature)> = Vec::new();

    let brute_start = Instant::now();

    // --- Brute Force Search ---
    for query in queries.iter() {
        let mut closest: (u32, Option<&HashFeature>) = (u32::MAX, None);

        for candidate in dataset.iter() {
            let diff = query.id.diff(&candidate.id, true) as u32;
            if diff < closest.0 {
                closest.0 = diff;
                closest.1 = Some(candidate);
            }
        }

        if let Some(hash) = closest.1 {
            brute_results.push((closest.0, hash));
        }
    }

    let brute_time: std::time::Duration = brute_start.elapsed();

    let hnsw_start = Instant::now();

    for n in &queries {
        let results: Vec<(u32, &HashFeature)> = model.knn_search(n, 24, 5);
        apo_results.push(results[0].clone());
    }

    let hnsw_time: std::time::Duration = hnsw_start.elapsed();

    let mut matches = 0;
    for i in 0..apo_results.len() {
        if apo_results[i].0 == brute_results[i].0 && apo_results[i].1.get_id().hash() == brute_results[i].1.get_id().hash() {
            matches += 1;
        }
    }

    println!("Matches: {}/{}", matches, apo_results.len());
    println!("Creation time: {:?}", creation_time);
    println!("Brute force time: {:?}", brute_time);
    println!("HNSW search time: {:?}", hnsw_time);
}
