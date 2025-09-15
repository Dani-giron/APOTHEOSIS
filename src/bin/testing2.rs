use apotheosis2::{controllers::hnsw::Hnsw, datalayer::features::NumberFeature};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use std::time::Instant;
use apotheosis2::datalayer::algorithms::NormalDistance;


pub fn main() {
    // Simple approach with StdRng
    let mut rng = StdRng::seed_from_u64(10); // Use same seed for reproducibility
    let random_numbers: Vec<u32> = (0..100000).map(|_| rng.gen_range(0..10_000_000)).collect();
    let queries: Vec<u32> = (0..1000).map(|_| rng.gen_range(0..10_000_000)).collect();

    println!("Random numbers: {:?}", random_numbers);
    let mut features: Vec<NumberFeature> = vec![];

    for n in random_numbers.clone() { // Clone to use later for brute force
        features.push(NumberFeature { id: n });
    }

    let creation_start = Instant::now();
    let mut model: Hnsw<NormalDistance, NumberFeature, 12, 24> = Hnsw::default();

    for f in features {
        model.insert(f);
    }

    println!("HNSW Model created in: {:?}", creation_start.elapsed());

    // Brute-force search for ground truth and timing
    let brute_start = Instant::now();
    let mut brute_results = Vec::new();
    for &query in &queries {
        let (idx, val) = random_numbers.iter().enumerate()
            .min_by_key(|(_, x)| x.abs_diff(query))
            .unwrap();
        let distance = query.abs_diff(*val);
        brute_results.push((*val, distance)); // Store (nearest_number, distance)
    }
    let brute_time: std::time::Duration = brute_start.elapsed();
    println!("Brute force search took: {:?}", brute_time);

    // Perform HNSW search and collect results
    let hnsw_start = Instant::now();
    let mut hnsw_results = Vec::new();
    for n in queries.clone() {
        let results = model.knn_search(NumberFeature { id: n }, 24, 5);
        // Get the best result (first one, closest)
        if let Some((best_feature, best_score)) = results.first() {
            hnsw_results.push((*best_feature, *best_score)); // Dereference both values
        } else {
            hnsw_results.push((0, u32::MAX)); // No result found
    }
}
    let hnsw_time = hnsw_start.elapsed();
    println!("HNSW search took: {:?}", hnsw_time);

    // Count exact matches
    let mut exact_matches = 0;
    let total_queries = queries.len();
    
    for (i, &query) in queries.iter().enumerate() {
        if i < brute_results.len() && i < hnsw_results.len() {
            let (brute_nearest, brute_distance) = brute_results[i];
            let (hnsw_nearest, hnsw_distance) = hnsw_results[i];
            
            // Check if both nearest number AND distance are exactly the same
            let is_exact_match = brute_nearest == hnsw_nearest && brute_distance == hnsw_distance;
            
            if is_exact_match {
                exact_matches += 1;
            }
            
            // Print detailed comparison for first few queries
            if i < 10 {
                println!("Query {}: {}", i, query);
                println!("  Brute force: nearest={}, distance={}", brute_nearest, brute_distance);
                println!("  HNSW:        nearest={}, distance={}", hnsw_nearest, hnsw_distance);
                println!("  Exact match: {}", is_exact_match);
                println!();
            }
        }
    }
    
    let exactness_percentage = (exact_matches as f64 / total_queries as f64) * 100.0;
    
    // Performance and exactness summary
    println!("\n=== PERFORMANCE COMPARISON ===");
    println!("Brute force time: {:?}", brute_time);
    println!("HNSW time: {:?}", hnsw_time);
    println!("Speedup: {:.2}x", brute_time.as_nanos() as f64 / hnsw_time.as_nanos() as f64);
    
    println!("\n=== EXACTNESS METRICS ===");
    println!("Exact matches: {}/{}", exact_matches, total_queries);
    println!("Exactness: {:.2}%", exactness_percentage);
    println!("Non-matches: {:.2}%", 100.0 - exactness_percentage);
}
