// Configuration matrix tests for APOTHEOSIS 2
//
// Verifies that fitness functions and API contracts hold across different
// compile-time configurations (M, M0, EF, HEURISTIC).

use apotheosis2::controllers::apotheosis::Apotheosis;
use apotheosis2::datalayer::algorithms::NormalDistance;
use apotheosis2::datalayer::record::SimpleNumberRecord;

// Default configuration (M=16, M0=32, EF=400, HEURISTIC=false).
type ApoNumDefault = Apotheosis<SimpleNumberRecord, NormalDistance>;

// Sparse graph - M=4, M0=8 reduce connectivity; EF=50 reduces search breadth.
// Exercises the HNSW code paths under low-connectivity conditions.
type ApoNumSmall = Apotheosis<SimpleNumberRecord, NormalDistance, 4, 8, 50>;

// Heuristic neighbor selection - HEURISTIC=true activates the alternate branch
// in insert(). Same M/M0/EF as default so only the selection algorithm differs.
type ApoNumHeuristic = Apotheosis<SimpleNumberRecord, NormalDistance, 16, 32, 400, true>;

// Deterministic dataset of 10 records (keys 0..10), identical for all configs.
fn build_dataset_default() -> ApoNumDefault {
    let mut idx = ApoNumDefault::new();
    for i in 0..10u32 {
        assert!(
            idx.insert(SimpleNumberRecord::create(i.to_string()).unwrap()),
            "insert unexpectedly returned false for key {i}"
        );
    }
    idx
}

fn build_dataset_small() -> ApoNumSmall {
    let mut idx = ApoNumSmall::new();
    for i in 0..10u32 {
        assert!(
            idx.insert(SimpleNumberRecord::create(i.to_string()).unwrap()),
            "insert unexpectedly returned false for key {i}"
        );
    }
    idx
}

fn build_dataset_heuristic() -> ApoNumHeuristic {
    let mut idx = ApoNumHeuristic::new();
    for i in 0..10u32 {
        assert!(
            idx.insert(SimpleNumberRecord::create(i.to_string()).unwrap()),
            "insert unexpectedly returned false for key {i}"
        );
    }
    idx
}

// ---------------------------------------------------------------------------
// FF1 - sync_invariant: records.len() == zero_layer node count after 10 inserts
//
// draw_model()[0].0.len() is the proxy for zero_layer node count.
// draw_model() is pub on Hnsw; [0] is always zero_layer (pushed first,
// unconditionally). .0 is Vec<usize> of feature indices - independent of M0.
//
// Full radix traversal (every key maps to a valid index) is not covered here
// because RadixNode exposes no public iterator; that constraint is enforced
// structurally by insert() being the sole mutation point.
// ---------------------------------------------------------------------------
#[test]
fn ff1_sync_invariant_default() {
    let idx = build_dataset_default();
    let zero_layer_count = idx.draw_model()[0].0.len();
    assert_eq!(idx.len(), 10, "records must contain 10 entries");
    assert_eq!(zero_layer_count, 10, "zero_layer must contain 10 nodes");
    assert_eq!(
        idx.len(),
        zero_layer_count,
        "records and zero_layer diverged"
    );
}

#[test]
fn ff1_sync_invariant_small() {
    let idx = build_dataset_small();
    let zero_layer_count = idx.draw_model()[0].0.len();
    assert_eq!(idx.len(), 10, "records must contain 10 entries");
    assert_eq!(zero_layer_count, 10, "zero_layer must contain 10 nodes");
    assert_eq!(
        idx.len(),
        zero_layer_count,
        "records and zero_layer diverged"
    );
}

#[test]
fn ff1_sync_invariant_heuristic() {
    let idx = build_dataset_heuristic();
    let zero_layer_count = idx.draw_model()[0].0.len();
    assert_eq!(idx.len(), 10, "records must contain 10 entries");
    assert_eq!(zero_layer_count, 10, "zero_layer must contain 10 nodes");
    assert_eq!(
        idx.len(),
        zero_layer_count,
        "records and zero_layer diverged"
    );
}

// ---------------------------------------------------------------------------
// FF4 - exact_match: searching an inserted key returns it with distance 0
//
// Key 5 is in the dataset → radix hit → fast-path → first result distance 0.
// ---------------------------------------------------------------------------
#[test]
fn ff4_exact_match_default() {
    let idx = build_dataset_default();
    let results = idx.search(&5u32, 1, None);
    assert!(
        !results.is_empty(),
        "search for inserted key 5 must return a result"
    );
    assert_eq!(results[0].0, 0, "exact match must have distance 0");
}

#[test]
fn ff4_exact_match_small() {
    let idx = build_dataset_small();
    let results = idx.search(&5u32, 1, None);
    assert!(
        !results.is_empty(),
        "search for inserted key 5 must return a result"
    );
    assert_eq!(results[0].0, 0, "exact match must have distance 0");
}

#[test]
fn ff4_exact_match_heuristic() {
    let idx = build_dataset_heuristic();
    let results = idx.search(&5u32, 1, None);
    assert!(
        !results.is_empty(),
        "search for inserted key 5 must return a result"
    );
    assert_eq!(results[0].0, 0, "exact match must have distance 0");
}

// ---------------------------------------------------------------------------
// FF6 - roundtrip: dump+load produces identical search results
//
// Distinct paths per config to avoid race conditions when cargo test runs
// integration tests in parallel.
// ---------------------------------------------------------------------------
#[test]
fn ff6_roundtrip_default() {
    use std::fs;

    let path = "target/test_config_ff6_default.bin";
    let idx = build_dataset_default();

    let before: Vec<u32> = idx
        .search(&7u32, 3, Some(50))
        .into_iter()
        .map(|(d, _)| d)
        .collect();

    idx.dump(path).expect("dump() must not fail");
    let loaded = ApoNumDefault::load(path).expect("load() must not fail");

    let after: Vec<u32> = loaded
        .search(&7u32, 3, Some(50))
        .into_iter()
        .map(|(d, _)| d)
        .collect();

    fs::remove_file(path).ok();
    assert_eq!(
        before, after,
        "search results must be identical after roundtrip"
    );
}

#[test]
fn ff6_roundtrip_small() {
    use std::fs;

    let path = "target/test_config_ff6_small.bin";
    let idx = build_dataset_small();

    let before: Vec<u32> = idx
        .search(&7u32, 3, Some(50))
        .into_iter()
        .map(|(d, _)| d)
        .collect();

    idx.dump(path).expect("dump() must not fail");
    let loaded = ApoNumSmall::load(path).expect("load() must not fail");

    let after: Vec<u32> = loaded
        .search(&7u32, 3, Some(50))
        .into_iter()
        .map(|(d, _)| d)
        .collect();

    fs::remove_file(path).ok();
    assert_eq!(
        before, after,
        "search results must be identical after roundtrip"
    );
}

#[test]
fn ff6_roundtrip_heuristic() {
    use std::fs;

    let path = "target/test_config_ff6_heuristic.bin";
    let idx = build_dataset_heuristic();

    let before: Vec<u32> = idx
        .search(&7u32, 3, Some(50))
        .into_iter()
        .map(|(d, _)| d)
        .collect();

    idx.dump(path).expect("dump() must not fail");
    // HEURISTIC is encoded in the header and validated by load(), so the
    // loading type must match the dumping one (ApoNumHeuristic here).
    let loaded = ApoNumHeuristic::load(path).expect("load() must not fail");

    let after: Vec<u32> = loaded
        .search(&7u32, 3, Some(50))
        .into_iter()
        .map(|(d, _)| d)
        .collect();

    fs::remove_file(path).ok();
    assert_eq!(
        before, after,
        "search results must be identical after roundtrip"
    );
}

// ---------------------------------------------------------------------------
// AC1 - ann_ordered: ANN path returns ≤k results sorted by distance ascending
//
// Key 9999 is not in the dataset → radix miss → knn_search() (ANN path).
// ---------------------------------------------------------------------------
#[test]
fn ac1_ann_ordered_default() {
    let idx = build_dataset_default();
    let k = 3;
    let results = idx.search(&9999u32, k, None);

    assert!(
        results.len() <= k,
        "must return at most k={k} results, got {}",
        results.len()
    );

    let distances: Vec<u32> = results.iter().map(|(d, _)| *d).collect();
    for w in distances.windows(2) {
        assert!(w[0] <= w[1], "results not sorted: {:?} > {:?}", w[0], w[1]);
    }
}

#[test]
fn ac1_ann_ordered_small() {
    let idx = build_dataset_small();
    let k = 3;
    let results = idx.search(&9999u32, k, None);

    assert!(
        results.len() <= k,
        "must return at most k={k} results, got {}",
        results.len()
    );

    let distances: Vec<u32> = results.iter().map(|(d, _)| *d).collect();
    for w in distances.windows(2) {
        assert!(w[0] <= w[1], "results not sorted: {:?} > {:?}", w[0], w[1]);
    }
}

#[test]
fn ac1_ann_ordered_heuristic() {
    let idx = build_dataset_heuristic();
    let k = 3;
    let results = idx.search(&9999u32, k, None);

    assert!(
        results.len() <= k,
        "must return at most k={k} results, got {}",
        results.len()
    );

    let distances: Vec<u32> = results.iter().map(|(d, _)| *d).collect();
    for w in distances.windows(2) {
        assert!(w[0] <= w[1], "results not sorted: {:?} > {:?}", w[0], w[1]);
    }
}

// ---------------------------------------------------------------------------
// AC - cross-config load: header validation rejects M/M0/EF mismatches
// ---------------------------------------------------------------------------

#[test]
fn ac_cross_config_load_rejects_m_mismatch() {
    let path = "target/test_config_cross_mismatch.bin";
    let _ = std::fs::remove_file(path);

    let idx = build_dataset_small(); // M=4, M0=8, EF=50
    idx.dump(path).expect("dump() must not fail"); // safe: build_dataset_small() succeeded

    let result = ApoNumDefault::load(path); // expects M=16, M0=32, EF=400

    let _ = std::fs::remove_file(path);

    assert!(result.is_err(), "load() must reject M/M0/EF mismatch");

    // Exact format from apotheosis.rs load(): "Model parameter mismatch. File has M={m}, M0={m0}, EF={ef} but code expects M={M}, M0={M0}, EF={EF}"
    let msg = result.err().unwrap().to_string();
    assert!(
        msg.contains("M=4") && msg.contains("M=16"),
        "error message must cite both file M=4 and expected M=16, got: {msg}"
    );
}

// dump() encodes HEURISTIC as the 17th header byte and load() validates it
// against the compile-time HEURISTIC of the loading type (issue #10).
// A file built with HEURISTIC=true must not load into a HEURISTIC=false type.
#[test]
fn ac_cross_config_load_rejects_heuristic_mismatch() {
    let path = "target/test_config_cross_heuristic.bin";
    let _ = std::fs::remove_file(path);

    let idx = build_dataset_heuristic(); // HEURISTIC=true, M=16, M0=32, EF=400
    idx.dump(path).expect("dump() must not fail");

    let result = ApoNumDefault::load(path); // HEURISTIC=false, same M/M0/EF

    let _ = std::fs::remove_file(path);

    assert!(result.is_err(), "load() must reject HEURISTIC mismatch");

    // Exact format from apotheosis.rs load(): "Model parameter mismatch. File has HEURISTIC={} but code expects HEURISTIC={}"
    let msg = result.err().unwrap().to_string();
    assert!(
        msg.contains("HEURISTIC=true") && msg.contains("HEURISTIC=false"),
        "error message must cite both file HEURISTIC=true and expected HEURISTIC=false, got: {msg}"
    );
}
