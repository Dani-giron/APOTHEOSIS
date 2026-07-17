// API contract tests for APOTHEOSIS 2
//
// A failing test here means a public operation violates its observable
// behavioral contract as seen by an external caller.

use apotheosis2::controllers::apotheosis::Apotheosis;
use apotheosis2::datalayer::algorithms::NormalDistance;
use apotheosis2::datalayer::record::SimpleNumberRecord;

type ApoNum = Apotheosis<SimpleNumberRecord, NormalDistance>;

// ---------------------------------------------------------------------------
// Test 1 - search_ann_path_returns_k_or_fewer_sorted_by_distance
//
// Contract: search() on the ANN path returns at most k results sorted by
// distance ascending. Querying a missing key (9999) forces a radix miss
// and invokes knn_search().
// ---------------------------------------------------------------------------
#[test]
fn search_ann_path_returns_k_or_fewer_sorted_by_distance() {
    let mut idx = ApoNum::new();
    for i in 0..20u32 {
        idx.insert(SimpleNumberRecord::create(i.to_string()));
    }

    let k = 5;
    let results = idx.search(&9999u32, k, None);

    assert!(
        results.len() <= k,
        "search() must return at most k={} results, got {}",
        k,
        results.len()
    );

    let distances: Vec<u32> = results.iter().map(|(d, _)| *d).collect();
    for w in distances.windows(2) {
        assert!(w[0] <= w[1], "results not sorted: {:?} > {:?}", w[0], w[1]);
    }
}

// ---------------------------------------------------------------------------
// Test 2 - search_on_empty_system_returns_empty_vec
//
// Contract: search() on an empty Apotheosis returns an empty Vec.
// ---------------------------------------------------------------------------
#[test]
fn search_on_empty_system_returns_empty_vec() {
    let idx = ApoNum::new();
    let results = idx.search(&0u32, 1, None);
    assert!(
        results.is_empty(),
        "search() on empty index must return empty Vec"
    );
}

// ---------------------------------------------------------------------------
// Test 3 - draw_produces_one_gexf_file_per_layer
//
// Contract: draw(base_path) creates exactly one .gexf file per HNSW layer,
// named <stem>_layer<N>.gexf for N in 0..L where L = draw_model().len().
// No extra files beyond layer L-1 may exist.
// ---------------------------------------------------------------------------
#[test]
fn draw_produces_one_gexf_file_per_layer() {
    use std::fs;
    use std::path::PathBuf;

    let dir = "target/test_draw_gexf";
    let base = format!("{}/model", dir);
    fs::create_dir_all(dir).unwrap();

    let mut idx = ApoNum::new();
    for i in 0..200u32 {
        idx.insert(SimpleNumberRecord::create(i.to_string()));
    }

    let layer_count = idx.hnsw.draw_model().len();
    idx.draw(&base);

    for n in 0..layer_count {
        let f = PathBuf::from(format!("{}/model_layer{}.gexf", dir, n));
        assert!(f.exists(), "draw() must produce {:?} for layer {}", f, n);
    }

    let extra = PathBuf::from(format!("{}/model_layer{}.gexf", dir, layer_count));
    assert!(
        !extra.exists(),
        "draw() must not produce a file for layer {} (only {} layers exist)",
        layer_count,
        layer_count
    );

    fs::remove_dir_all(dir).ok();
}

// ---------------------------------------------------------------------------
// Test 4 - load_overwrites_previous_in_memory_state
//
// Contract: load() replaces all in-memory state with the deserialized model.
// Data present before load() must not be reachable afterwards; data from the
// loaded model must be.
// ---------------------------------------------------------------------------
#[test]
fn load_overwrites_previous_in_memory_state() {
    use apotheosis2::datalayer::record::ApotheosisRecord;
    use std::fs;

    let path = "target/test_load_overwrite.bin";

    let mut idx_a = ApoNum::new();
    for i in 0..5u32 {
        idx_a.insert(SimpleNumberRecord::create(i.to_string()));
    }
    idx_a.dump(path).expect("dump() must not fail");

    let mut idx_b = ApoNum::new();
    for i in 100..103u32 {
        idx_b.insert(SimpleNumberRecord::create(i.to_string()));
    }
    idx_b = ApoNum::load(path).expect("load() must not fail");

    assert!(
        !idx_b.search(&2u32, 1, None).is_empty(),
        "after load(), loaded data (key 2) must be reachable"
    );

    let has_stale = idx_b
        .search(&100u32, 5, None)
        .iter()
        .any(|(_, r)| r.search_id() == 100u32);
    assert!(
        !has_stale,
        "after load(), pre-load data (key 100) must be gone"
    );

    fs::remove_file(path).ok();
}

// ---------------------------------------------------------------------------
// Test 5 - load_rejects_corrupt_magic_bytes
//
// Contract: load() must return Err (not panic) when the file does not start
// with the expected "APOT" magic bytes.
// ---------------------------------------------------------------------------
#[test]
fn load_rejects_corrupt_magic_bytes() {
    use std::fs;

    let path = "target/test_corrupt_magic.bin";
    // Write a full 17-byte header with wrong magic bytes but otherwise plausible
    // content, so the magic check (not a short read) is what rejects the file.
    fs::write(
        path,
        b"NOPE\x10\x00\x00\x00\x20\x00\x00\x00\x90\x01\x00\x00\x00",
    )
    .unwrap(); // safe: simple fs write

    let result = ApoNum::load(path);
    fs::remove_file(path).ok();

    assert!(
        result.is_err(),
        "load() must return Err on invalid magic bytes"
    );
    let msg = result.err().unwrap().to_string();
    assert!(
        msg.contains("magic") || msg.contains("Invalid"),
        "error message must mention invalid file, got: {msg}"
    );
}

// ---------------------------------------------------------------------------
// Test 6 - load_rejects_truncated_file
//
// Contract: load() must return Err (not panic) when the file is shorter than
// the 17-byte header it expects.
// ---------------------------------------------------------------------------
#[test]
fn load_rejects_truncated_file() {
    use std::fs;

    let path = "target/test_truncated.bin";
    fs::write(path, b"APOT\x10\x00").unwrap(); // safe: 6 bytes - shorter than 17-byte header

    let result = ApoNum::load(path);
    fs::remove_file(path).ok();

    assert!(
        result.is_err(),
        "load() must return Err on truncated header"
    );
}

// ---------------------------------------------------------------------------
// Test 7 - search_with_k_zero_returns_empty
//
// Contract: search() with k=0 must return an empty Vec without panicking.
// Query not in dataset forces the ANN path; .take(0) returns empty.
// ---------------------------------------------------------------------------
#[test]
fn search_with_k_zero_returns_empty() {
    let mut idx = ApoNum::new();
    for i in 0..10u32 {
        idx.insert(SimpleNumberRecord::create(i.to_string())); // safe: unique keys
    }
    // Query not in dataset → ANN path → .take(0) → empty Vec.
    let results = idx.search(&9999u32, 0, None);
    assert!(
        results.is_empty(),
        "search() with k=0 must return empty Vec, got {} results",
        results.len()
    );
}

// ---------------------------------------------------------------------------
// Test 8 - search_with_k_greater_than_n_returns_all
//
// Contract: search() with k > records.len() must return at most records.len()
// results without panicking. Requesting more neighbors than exist is valid.
// ---------------------------------------------------------------------------
#[test]
fn search_with_k_greater_than_n_returns_all() {
    let mut idx = ApoNum::new();
    for i in 0..10u32 {
        idx.insert(SimpleNumberRecord::create(i.to_string())); // safe: unique keys
    }
    let k = 100; // far larger than the 10 records
    let results = idx.search(&9999u32, k, None);
    assert!(
        results.len() <= 10,
        "search() with k={k} on 10 records must return at most 10, got {}",
        results.len()
    );
    assert!(
        !results.is_empty(),
        "search() with k={k} on non-empty index must return something"
    );
}
