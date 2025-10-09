use apotheosis2::controllers::apotheosis::Apotheosis;
use apotheosis2::datalayer::algorithms::NormalDistance;
use apotheosis2::datalayer::features::FeatureType;
use apotheosis2::{datalayer::features::NumberFeature};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use std::u32::MAX;
use std::time::Instant;

// cargo run --bin testing
pub fn main() {
    // Simple approach with thread_rng
    let mut rng = StdRng::seed_from_u64(42); // Use same seed for reproducibility
    let random_numbers: Vec<u32> = (0..8000).map(|_| rng.gen_range(0..100_000_000)).collect();
    let random_queries: Vec<u32> = (0..5800).map(|_| rng.gen_range(0..100_000_000)).collect();
    //random_numbers = vec![13, 52, 54, 63, 99, 40, 3, 61, 41, 34];
    //println!("Base: {:?}", random_numbers);

    //let queries: Vec<u32> = (0..100).map(|_| rng.random_range(0..10_000_000)).collect();

    //print<ln!("{:?}", random_numbers);
    let mut features: Vec<NumberFeature> = vec![];

    println!("Queries: {:?}", random_queries);

    for &n in &random_numbers {
        if n == 6 {
            println!("Inserting special number 6");
        }
        features.push( NumberFeature::create(n.to_string()) )
    }

    let creation_start = Instant::now();
    //let mut model: Hnsw<NormalDistance, NumberFeature> = Hnsw::default();

    let mut model = Apotheosis::<NumberFeature, NormalDistance, (), 32, 64, 64>::new();

    for f in features {
        model.insert(f, ());
    }
    let creation_time: std::time::Duration = creation_start.elapsed();

    //model.print_layers();
    println!("HNSW Model created in: {:?}", creation_time);

    
    let queries: Vec<u32> = [10, 15, 21 , 30, 40, 50, 60, 70, 80, 90].to_vec();
    let mut all_results = Vec::new();

    // Brute force search for neighbors
    let mut closest = (MAX, MAX);
    let mut brute_results: Vec<(u32, u32)> = vec![];
    for n in random_queries.iter() {
        for n2 in &random_numbers {
            let dist = if n2 > n { n2 - n } else { n - n2 };
            if dist < closest.0 {
                closest.0 = dist;
                closest.1 = *n2;
            }
        }
        brute_results.push(closest);
        closest = (MAX, MAX);
    }

    //println!("{:?}", brute_results);


    let query_start = Instant::now();


    for n in random_queries.iter() {
        let results = model.search(n.to_string(), 1);
        all_results.push(results[0]);
    }

    let query_time: std::time::Duration = query_start.elapsed();


    println!("{:?}", all_results);

    let mut matches = 0;
    for i in 0..all_results.len() {
        if all_results[i].0 == brute_results[i].0 && all_results[i].1 == &brute_results[i].1 {
            matches += 1;
        }
    }

    println!("Matches: {}/{}", matches, all_results.len());
    println!("Queries took: {:?}", query_time);
    
}
