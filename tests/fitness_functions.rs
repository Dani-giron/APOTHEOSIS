// Fitness functions - architectural invariant tests for APOTHEOSIS 2
//
// A failing
// test here means an architectural invariant is broken, not just a bug.

use apotheosis2::controllers::apotheosis::Apotheosis;
use apotheosis2::datalayer::algorithms::NormalDistance;
use apotheosis2::datalayer::record::SimpleNumberRecord;

// Convenience alias with default const generics (M=16, M0=32, EF=400, HEURISTIC=false)
type ApoNum = Apotheosis<SimpleNumberRecord, NormalDistance>;

// ---------------------------------------------------------------------------
// Test 1 - sync_invariant_holds_after_bulk_insert
//
// Invariant: after N successful inserts of distinct keys, records.len()
// and the HNSW element count must both equal N.
//
// The three parallel structures (records[], hnsw.features[], radix) must
// stay in sync at all times. This is documented in:
//   - docs/architecture/views/module/vista-modulos.md §7.2 (sync invariant)
//   - docs/architecture/quality-attributes.md (correctness attribute)
// ---------------------------------------------------------------------------
#[test]
fn sync_invariant_holds_after_bulk_insert() {
    let mut idx = ApoNum::new();

    for i in 0..50u32 {
        idx.insert(SimpleNumberRecord::create(i.to_string()));
    }

    let hnsw_len = idx.hnsw.draw_model()[0].0.len();

    assert_eq!(
        idx.records.len(),
        50,
        "records[] should contain exactly 50 entries after 50 inserts"
    );
    assert_eq!(
        hnsw_len, 50,
        "HNSW zero_layer should contain exactly 50 nodes after 50 inserts"
    );
    assert_eq!(
        idx.records.len(),
        hnsw_len,
        "records[] and HNSW must be in sync - parallel structures diverged"
    );
}

// ---------------------------------------------------------------------------
// Test 2 - insert_returns_true_for_new_keys
//
// Invariant: insert() returns true when a new unique key is inserted.
// A return value of false signals a duplicate rejection; true signals
// successful indexing.
//
// Ref: docs/architecture/interfaz-apotheosis.md §2.1.2
// ---------------------------------------------------------------------------
#[test]
fn insert_returns_true_for_new_keys() {
    let mut idx = ApoNum::new();
    let result = idx.insert(SimpleNumberRecord::create("1".to_string()));
    assert!(result, "insert() must return true for a new unique key");
}

// ---------------------------------------------------------------------------
// Test 3 - insert_returns_false_for_duplicate_keys
//
// Invariant: insert() returns false when a key that already exists in the
// radix tree is re-inserted, preventing duplicate entries.
//
// Ref: docs/architecture/interfaz-apotheosis.md §2.1.2
//
// NOTE: this test does NOT assert sync invariant after the duplicate insert.
// Kept focused on the return-value contract only; sync state after rejection
// is covered by sync_invariant_holds_after_duplicate_insert.
// ---------------------------------------------------------------------------
#[test]
fn insert_returns_false_for_duplicate_keys() {
    let mut idx = ApoNum::new();

    let first = idx.insert(SimpleNumberRecord::create("42".to_string()));
    assert!(first, "first insert of key '42' must return true");

    let second = idx.insert(SimpleNumberRecord::create("42".to_string()));
    assert!(!second, "duplicate insert of key '42' must return false");
}

// ---------------------------------------------------------------------------
// Test 4 - search_returns_exact_match_via_fast_path
//
// Invariant: searching for a key that was inserted returns that exact key
// as the first result with distance 0, via the radix tree fast-path.
//
// When the query has a radix key and it exists in the tree, Apotheosis
// calls get_neighbors_node() which pushes the matched node first with
// distance 0. The ANN path is bypassed entirely.
//
// Ref: docs/architecture/views/cc/vista-cc.md (fast-path vs ANN path)
//      docs/architecture/interfaz-apotheosis.md §2.1.3
// ---------------------------------------------------------------------------
#[test]
fn search_returns_exact_match_via_fast_path() {
    let mut idx = ApoNum::new();
    idx.insert(SimpleNumberRecord::create("100".to_string()));

    let query: u32 = 100;
    let results = idx.search(&query, 1, None);

    assert!(
        !results.is_empty(),
        "search() must return at least one result when the key exists"
    );
    assert_eq!(
        results[0].0, 0,
        "exact match via fast-path must have distance 0"
    );
}

// ---------------------------------------------------------------------------
// Test 5 - search_returns_k_or_fewer_results
//
// Invariant: search() never returns more than k results regardless of
// how many records are in the model.
//
// Ref: docs/architecture/interfaz-apotheosis.md §2.1.3
// ---------------------------------------------------------------------------
#[test]
fn search_returns_k_or_fewer_results() {
    let mut idx = ApoNum::new();

    for i in 0..10u32 {
        idx.insert(SimpleNumberRecord::create(i.to_string()));
    }

    let query: u32 = 5;
    let results = idx.search(&query, 3, None);

    assert!(
        results.len() <= 3,
        "search() must return at most k=3 results, got {}",
        results.len()
    );
}

// ---------------------------------------------------------------------------
// Test 6 - dump_load_roundtrip_preserves_search_results
//
// Invariant: a model serialized with dump() and reloaded with load()
// produces identical search results for the same query.
//
// This protects the persistence integrity quality attribute: a stored
// model must be bit-for-bit equivalent in behavior to the in-memory one.
//
// Ref: docs/architecture/quality-attributes.md (reproducibility)
//      docs/architecture/interfaz-apotheosis.md §2.1.4, §2.1.5
// ---------------------------------------------------------------------------
#[test]
fn dump_load_roundtrip_preserves_search_results() {
    use std::fs;

    let path = "target/test_roundtrip.bin";

    let mut idx = ApoNum::new();
    for i in 0..20u32 {
        idx.insert(SimpleNumberRecord::create(i.to_string()));
    }

    let query: u32 = 7;
    let original_results: Vec<u32> = idx
        .search(&query, 3, Some(50))
        .into_iter()
        .map(|(d, _)| d)
        .collect();

    idx.dump(path).expect("dump() must not fail");

    let loaded = ApoNum::load(path).expect("load() must not fail");

    let loaded_results: Vec<u32> = loaded
        .search(&query, 3, Some(50))
        .into_iter()
        .map(|(d, _)| d)
        .collect();

    fs::remove_file(path).ok();

    assert_eq!(
        original_results, loaded_results,
        "search results after load() must be identical to pre-dump results"
    );
}

// ---------------------------------------------------------------------------
// Test 7 - sync_invariant_holds_after_duplicate_insert
//
// Invariant: a duplicate insert (key already in radix tree) must return false
// AND must not grow records[], hnsw.features[], or zero_layer.
//
// fitness_functions.rs::insert_returns_false_for_duplicate_keys (Test 3)
// explicitly skips verifying sync state after the duplicate. This test closes
// that gap: the three parallel structures must stay at len=1 after the dup.
// ---------------------------------------------------------------------------
#[test]
fn sync_invariant_holds_after_duplicate_insert() {
    let mut idx = ApoNum::new();

    let first = idx.insert(SimpleNumberRecord::create("7".to_string()));
    assert!(first, "first insert must return true");

    let second = idx.insert(SimpleNumberRecord::create("7".to_string()));
    assert!(!second, "duplicate insert must return false");

    let zero_layer_count = idx.hnsw.draw_model()[0].0.len();
    assert_eq!(
        idx.records.len(),
        1,
        "records must not grow on duplicate insert"
    );
    assert_eq!(
        zero_layer_count, 1,
        "zero_layer must not grow on duplicate insert"
    );
    assert_eq!(
        idx.records.len(),
        zero_layer_count,
        "records and zero_layer diverged after duplicate"
    );
}

// ---------------------------------------------------------------------------
// Test 8 - sync_invariant_maps_every_key_to_its_own_record
//
// Invariant: the shared index actually corresponds. The length-based sync
// tests above pass even if entries are crossed (records[3] holding the data
// of key 7). Exact-match search for every inserted key must return the
// record that was inserted under that key, with distance 0.
// ---------------------------------------------------------------------------
#[test]
fn sync_invariant_maps_every_key_to_its_own_record() {
    use apotheosis2::datalayer::record::ApotheosisRecord;

    let mut idx = ApoNum::new();
    let n = 100u32;
    for i in 0..n {
        idx.insert(SimpleNumberRecord::create(i.to_string()));
    }

    for key in 0..n {
        let results = idx.search(&key, 1, None);
        assert!(
            !results.is_empty(),
            "exact search for key {key} returned nothing"
        );
        let (distance, record) = &results[0];
        assert_eq!(
            *distance, 0,
            "exact match for key {key} must have distance 0"
        );
        assert_eq!(
            record.search_id(),
            key,
            "index desync: searching key {key} returned record for key {}",
            record.search_id()
        );
    }
}
