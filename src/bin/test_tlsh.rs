use apotheosis2::controllers::apotheosis::Apotheosis;
use apotheosis2::datalayer::algorithms::{TlshDistance};
use apotheosis2::datalayer::features::{FeatureType, TlshHashFeature};
use tlsh2::TlshDefault;
use tracing::info;
use std::str::FromStr;
use std::time::Instant;
use serde_json::Value;
use std::fs;


fn read_hashes_from_json<P: AsRef<std::path::Path>>(path: P) -> Vec<String> {
    let data = fs::read_to_string(path).expect("Failed to read JSON file");
    let v: Value = serde_json::from_str(&data).expect("Failed to parse JSON");
    let obj = v.as_object().expect("Expected JSON object at root");

    if let Some(hashes_value) = obj.get("hashes") {
        if let Some(hashes_array) = hashes_value.as_array() {
            return hashes_array
                .iter()
                .filter_map(|val| val.as_str().map(|s| s.to_string()))
                .collect();
        }
    }

    Vec::new()
}


fn create_tlsh_object(hash: String) -> TlshDefault {
    TlshDefault::from_str(&hash).unwrap()
}

// cargo run --bin testing
pub fn main() {

    let hashes = read_hashes_from_json("output_hashes.json");
    println!("Number of hashes: {:?}", hashes.len());

    //let data = create_tlsh_objects(&hashes);

    // Initialize vectors before pushing
    let dataset: Vec<String> = hashes[..60000].to_vec();
    let dataset_copy: Vec<String> = dataset.clone();
    let queries: Vec<String> = hashes[1000000..1001000].to_vec();
    let mut apotheosis = Apotheosis::<TlshHashFeature, TlshDistance, (), 32, 64, 64>::new();
    let creation_start: Instant = Instant::now();

    println!("Dataset size: {}, Queries size: {}", dataset.len(), queries.len());
    println!("Starting insertion into HNSW model...");
    for f in dataset_copy {
        apotheosis.insert(TlshHashFeature::create(f), ());
    }

    let result = apotheosis.search("T1008100007FFA5C48F0F33EB5AEB455158576FE205AB2CA6D51A4828F24B2B408961F3B".to_string(), 1);
    println!("Distance: {}", result[0].0);
    let creation_time: std::time::Duration = creation_start.elapsed();

    let mut brute_results: Vec<(u32, TlshDefault)> = Vec::new();
    let mut apo_results: Vec<(u32, &TlshDefault, _)> = Vec::new();

    let brute_start = Instant::now();
    
    
    println!("Starting brute force search...");
    // --- Brute Force Search ---
    for query in queries.iter() {
        let mut closest: (u32, Option<&String>) = (u32::MAX, None);
        let tlsh_query = create_tlsh_object(query.to_string());
        for candidate in dataset.iter() {
            let tlsh_candidate = create_tlsh_object(candidate.to_string());

            let diff = tlsh_query.diff(&tlsh_candidate, true) as u32;
            if diff < closest.0 {
                closest.0 = diff;
                closest.1 = Some(&candidate);
            }
        }

        if let Some(hash) = closest.1 {
            let obj = create_tlsh_object(hash.to_string());
            brute_results.push((closest.0, obj));
        }
    }

    let brute_time: std::time::Duration = brute_start.elapsed();

    let hnsw_start = Instant::now();

    println!("Starting APOTHEOSIS search...");

    for n in &queries {
        let results: Vec<(u32, &TlshDefault, _)> = apotheosis.search(n.to_string(), 42);
        apo_results.push(results[0].clone());
    }

    let hnsw_time: std::time::Duration = hnsw_start.elapsed();

    let mut matches = 0;
    for i in 0..apo_results.len() {
        if apo_results[i].0 == brute_results[i].0 && apo_results[i].1.hash() == brute_results[i].1.hash() {
            matches += 1;
        }
    }

    println!("Matches: {}/{}", matches, apo_results.len());
    println!("Creation time: {:?}", creation_time);
    println!("Brute force time: {:?}", brute_time);
    println!("HNSW search time: {:?}", hnsw_time);
    
}
