// Boundary and stress configuration tests for APOTHEOSIS 2
//
// Probes structural limits: minimum viable parameters, inverted ratios,
// degenerate EF values, and runtime mismatches between k and ef_search.
//
// WHY M=2 AND NOT M=1
// -------------------
// M=1 causes OOM/hang on the first insert, not a catchable panic.
// random_level() computes -ln(u)/ln(M). With M=1, ln(1)=0 → f64::INFINITY
// → as usize = usize::MAX. initialize() then loops usize::MAX iterations.
// M=2 is the minimum viable value: ln(2) ≈ 0.693 → finite level values.

use apotheosis2::controllers::apotheosis::Apotheosis;
use apotheosis2::datalayer::algorithms::NormalDistance;
use apotheosis2::datalayer::record::SimpleNumberRecord;

// M=2 (minimum viable, see note above), M0=4, EF=1 (absolute minimum exploration).
// Graph built with near-zero exploration - structurally valid, low quality by design.
type ApoMinViable = Apotheosis<SimpleNumberRecord, NormalDistance, 2, 4, 1>;

// M > M0 (inverted standard ratio - normally M0 = 2×M per HNSW convention).
// Upper layers denser than zero_layer. No code path enforces M <= M0.
type ApoInverted = Apotheosis<SimpleNumberRecord, NormalDistance, 32, 8, 400>;

// M == M0 (no asymmetry between layers).
type ApoEqualMM0 = Apotheosis<SimpleNumberRecord, NormalDistance, 8, 8, 200>;

// Maximum connectivity - near brute-force for small datasets.
type ApoDense = Apotheosis<SimpleNumberRecord, NormalDistance, 64, 128, 1000>;

// Default alias reused for runtime-parameter edge cases (ef_search, k).
type ApoDefault = Apotheosis<SimpleNumberRecord, NormalDistance>;

// Small neighbor lists (M=4, M0=8) for the saturation/eviction test: with a
// dataset far larger than M0, every list fills and the eviction branch runs.
type ApoNumSmallSat = Apotheosis<SimpleNumberRecord, NormalDistance, 4, 8, 100>;

// ---------------------------------------------------------------------------
// Sync invariant × 4 boundary configs
//
// For degenerate configs the sync invariant must hold regardless of graph
// quality - records.len() == zero_layer node count after N inserts.
// ---------------------------------------------------------------------------

#[test]
fn sync_invariant_min_viable() {
    let mut idx = ApoMinViable::new();
    for i in 0..10u32 {
        idx.insert(SimpleNumberRecord::create(i.to_string()).unwrap()); // safe: keys are unique
    }
    let zero_len = idx.draw_model()[0].0.len();
    assert_eq!(idx.len(), 10);
    assert_eq!(zero_len, 10);
    assert_eq!(idx.len(), zero_len);
}

#[test]
fn sync_invariant_inverted_ratio() {
    let mut idx = ApoInverted::new();
    for i in 0..10u32 {
        idx.insert(SimpleNumberRecord::create(i.to_string()).unwrap()); // safe: keys are unique
    }
    let zero_len = idx.draw_model()[0].0.len();
    assert_eq!(idx.len(), 10);
    assert_eq!(zero_len, 10);
    assert_eq!(idx.len(), zero_len);
}

#[test]
fn sync_invariant_equal_m_m0() {
    let mut idx = ApoEqualMM0::new();
    for i in 0..10u32 {
        idx.insert(SimpleNumberRecord::create(i.to_string()).unwrap()); // safe: keys are unique
    }
    let zero_len = idx.draw_model()[0].0.len();
    assert_eq!(idx.len(), 10);
    assert_eq!(zero_len, 10);
    assert_eq!(idx.len(), zero_len);
}

#[test]
fn sync_invariant_dense() {
    let mut idx = ApoDense::new();
    for i in 0..10u32 {
        idx.insert(SimpleNumberRecord::create(i.to_string()).unwrap()); // safe: keys are unique
    }
    let zero_len = idx.draw_model()[0].0.len();
    assert_eq!(idx.len(), 10);
    assert_eq!(zero_len, 10);
    assert_eq!(idx.len(), zero_len);
}

// ---------------------------------------------------------------------------
// No-panic search × 4 boundary configs
//
// These configs are not expected to yield high-quality ANN results (especially
// ApoMinViable with EF=1 construction). The contract here is weaker: search()
// must not panic and must return at most k results sorted ascending.
// ---------------------------------------------------------------------------

#[test]
fn search_no_panic_min_viable() {
    let mut idx = ApoMinViable::new();
    for i in 0..10u32 {
        idx.insert(SimpleNumberRecord::create(i.to_string()).unwrap()); // safe: keys are unique
    }
    let k = 3;
    // Query not in dataset → ANN path. ef_search=None → unwrap_or(24) inside search().
    let results = idx.search(&9999u32, k, None);
    assert!(
        results.len() <= k,
        "must return at most k={k}, got {}",
        results.len()
    );
    let distances: Vec<u32> = results.iter().map(|(d, _)| *d).collect();
    for w in distances.windows(2) {
        assert!(w[0] <= w[1], "results not sorted: {} > {}", w[0], w[1]);
    }
}

#[test]
fn search_no_panic_inverted_ratio() {
    let mut idx = ApoInverted::new();
    for i in 0..10u32 {
        idx.insert(SimpleNumberRecord::create(i.to_string()).unwrap()); // safe: keys are unique
    }
    let k = 3;
    let results = idx.search(&9999u32, k, None);
    assert!(
        results.len() <= k,
        "must return at most k={k}, got {}",
        results.len()
    );
    let distances: Vec<u32> = results.iter().map(|(d, _)| *d).collect();
    for w in distances.windows(2) {
        assert!(w[0] <= w[1], "results not sorted: {} > {}", w[0], w[1]);
    }
}

#[test]
fn search_no_panic_equal_m_m0() {
    let mut idx = ApoEqualMM0::new();
    for i in 0..10u32 {
        idx.insert(SimpleNumberRecord::create(i.to_string()).unwrap()); // safe: keys are unique
    }
    let k = 3;
    let results = idx.search(&9999u32, k, None);
    assert!(
        results.len() <= k,
        "must return at most k={k}, got {}",
        results.len()
    );
    let distances: Vec<u32> = results.iter().map(|(d, _)| *d).collect();
    for w in distances.windows(2) {
        assert!(w[0] <= w[1], "results not sorted: {} > {}", w[0], w[1]);
    }
}

#[test]
fn search_no_panic_dense() {
    let mut idx = ApoDense::new();
    for i in 0..10u32 {
        idx.insert(SimpleNumberRecord::create(i.to_string()).unwrap()); // safe: keys are unique
    }
    let k = 3;
    let results = idx.search(&9999u32, k, None);
    assert!(
        results.len() <= k,
        "must return at most k={k}, got {}",
        results.len()
    );
    let distances: Vec<u32> = results.iter().map(|(d, _)| *d).collect();
    for w in distances.windows(2) {
        assert!(w[0] <= w[1], "results not sorted: {} > {}", w[0], w[1]);
    }
}

// ---------------------------------------------------------------------------
// Runtime edge cases - parametric, reuse ApoDefault
//
// These test the ef_search and k runtime parameters, not the compile-time
// const-generics. ApoDefault (M=16, M0=32, EF=400) is the vehicle.
// ---------------------------------------------------------------------------

// ef_search=Some(1000) with only 10 records in the index.
// search_layer_zero exhausts all candidates before reaching ef; must not panic.
#[test]
fn search_ef_search_greater_than_n() {
    let mut idx = ApoDefault::new();
    for i in 0..10u32 {
        idx.insert(SimpleNumberRecord::create(i.to_string()).unwrap()); // safe: keys are unique
    }
    let k = 5;
    let results = idx.search(&9999u32, k, Some(1000));
    assert!(
        results.len() <= k,
        "must return at most k={k}, got {}",
        results.len()
    );
    let distances: Vec<u32> = results.iter().map(|(d, _)| *d).collect();
    for w in distances.windows(2) {
        assert!(w[0] <= w[1], "results not sorted: {} > {}", w[0], w[1]);
    }
}

// k=10 > ef_search=2. knn_search explores only 2 candidates, returns ≤2,
// then .take(10) takes what's there. Must not panic; result ≤ k.
#[test]
fn search_k_greater_than_ef_search() {
    let mut idx = ApoDefault::new();
    for i in 0..20u32 {
        idx.insert(SimpleNumberRecord::create(i.to_string()).unwrap()); // safe: keys are unique
    }
    let k = 10;
    let results = idx.search(&9999u32, k, Some(2));
    assert!(
        results.len() <= k,
        "must return at most k={k}, got {}",
        results.len()
    );
    let distances: Vec<u32> = results.iter().map(|(d, _)| *d).collect();
    for w in distances.windows(2) {
        assert!(w[0] <= w[1], "results not sorted: {} > {}", w[0], w[1]);
    }
}

// ---------------------------------------------------------------------------
// Neighbor-list saturation: dataset much larger than M0
//
// With 10-record datasets and M0=32 the neighbor lists never fill up, so the
// replacement branch in connect_neighbors (list full: evict the farthest
// neighbor) never runs. 300 records with M0=8 saturate every list many times
// over, exercising eviction continuously. The graph must stay consistent:
// exact-match works for every key and ANN search still finds true nearest.
// ---------------------------------------------------------------------------
#[test]
fn eviction_under_saturation_keeps_graph_consistent() {
    use apotheosis2::datalayer::record::ApotheosisRecord;

    // M=4, M0=8: every node's list overflows repeatedly with 300 inserts.
    let mut idx = ApoNumSmallSat::new();
    let n = 300u32;
    for i in 0..n {
        idx.insert(SimpleNumberRecord::create(i.to_string()).unwrap());
    }

    // Every key must still resolve exactly through the radix fast-path.
    for key in (0..n).step_by(7) {
        let results = idx.search(&key, 1, None);
        assert!(
            !results.is_empty(),
            "exact search for key {key} returned nothing"
        );
        assert_eq!(
            results[0].0, 0,
            "exact match for key {key} must have distance 0"
        );
        assert_eq!(results[0].1.search_id(), key, "wrong record for key {key}");
    }

    // ANN path (query not in dataset) must still find the true nearest.
    let results = idx.search(&1000u32, 5, Some(100));
    assert!(!results.is_empty(), "ANN search returned nothing");
    let min = results.iter().map(|(d, _)| *d).min().unwrap();
    // True nearest to 1000 is 299, distance 701.
    assert_eq!(min, 701, "ANN nearest after heavy eviction must be exact");
}
