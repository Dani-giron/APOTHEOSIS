// ANN correctness tests for APOTHEOSIS 2
//
// A failing test here means the ANN graph returns wrong neighbors, not just
// that an API contract or invariant is violated.

use apotheosis2::controllers::apotheosis::Apotheosis;
use apotheosis2::datalayer::algorithms::NormalDistance;
use apotheosis2::datalayer::record::{ApotheosisRecord, SimpleNumberRecord};
use std::collections::HashSet;

type ApoNum = Apotheosis<SimpleNumberRecord, NormalDistance>;
type ApoNumHeuristic = Apotheosis<SimpleNumberRecord, NormalDistance, 16, 32, 400, true>;

// ---------------------------------------------------------------------------
// Test - ann_recall_matches_brute_force
//
// Invariant: the ANN path (knn_search) must agree with brute-force k-NN to a
// reasonable recall. This is the actual correctness property of the library;
// existing tests only check that search() doesn't crash and returns sorted,
// bounded-length results - never that the results are *right*.
//
// 300 records (even values 0..600 step 2) force multiple HNSW layers
// (random_level() with M=16 puts ~6% of inserts above layer 0; seed is fixed
// at 42 in default_rng(), so this is deterministic, not flaky).
//
// Query 301 is not in the dataset (odd, mid-range) so search() takes the ANN
// path (knn_search), not the radix fast-path.
// ---------------------------------------------------------------------------
#[test]
fn ann_recall_matches_brute_force() {
    let mut idx = ApoNum::new();
    let mut inserted = vec![];
    for i in 0..300u32 {
        let value = i * 2;
        idx.insert(SimpleNumberRecord::create(value.to_string()));
        inserted.push(value);
    }

    let query = 301u32;
    let k = 10;

    let ann_results = idx.search(&query, k, Some(400));
    let ann_values: HashSet<u32> = ann_results.iter().map(|(_, r)| r.search_id()).collect();

    let mut brute: Vec<(u32, u32)> = inserted.iter().map(|&v| (v.abs_diff(query), v)).collect();
    brute.sort_by_key(|&(d, _)| d);
    let brute_values: HashSet<u32> = brute.iter().take(k).map(|&(_, v)| v).collect();

    let recall = ann_values.intersection(&brute_values).count() as f64 / k as f64;
    assert!(
        recall >= 0.7,
        "ANN recall too low: {:.2} (ann={:?}, brute_top10={:?})",
        recall,
        ann_values,
        brute_values
    );

    // distance-0 sanity for the closest known points (catches gross errors,
    // e.g. wrong layer/index resolution returning unrelated neighbors)
    let brute_min_distance = brute[0].0;
    let ann_min_distance = ann_results
        .iter()
        .map(|(d, _)| *d)
        .min()
        .expect("ANN must return at least one result for a non-empty index");
    assert_eq!(
        ann_min_distance, brute_min_distance,
        "ANN closest distance must match brute-force closest distance"
    );
}

// ---------------------------------------------------------------------------
// Test - multi_layer_search_correct_across_layers
//
// zero_layer neighbors are feature indices; upper_layer neighbors are
// in-layer node indices. This asymmetry is load-bearing: a bug there would
// go undetected by tests using small datasets (10-20 records) that may never
// produce upper layers. 300 records guarantees multiple layers.
//
// First assert that the dataset actually produced upper layers; if it
// didn't, the test below would pass vacuously and prove nothing.
// ---------------------------------------------------------------------------
#[test]
fn multi_layer_search_correct_across_layers() {
    let mut idx = ApoNum::new();
    let mut inserted = vec![];
    for i in 0..300u32 {
        let value = i * 3;
        idx.insert(SimpleNumberRecord::create(value.to_string()));
        inserted.push(value);
    }

    let layer_count = idx.hnsw.draw_model().len();
    assert!(
        layer_count > 1,
        "dataset must produce upper layers to exercise the zero/upper index asymmetry, got {layer_count} layer(s)"
    );

    // Exact match (fast-path, bypasses traversal entirely) must still resolve
    // correctly regardless of how many upper layers exist.
    let exact = idx.search(&150u32, 1, None);
    assert!(
        !exact.is_empty(),
        "exact match for key 150 must return a result"
    );
    assert_eq!(exact[0].0, 0, "exact match must have distance 0");

    // ANN path must descend through upper layers correctly and land on the
    // true nearest neighbor at layer 0.
    let query = 301u32;
    let k = 5;
    let ann_results = idx.search(&query, k, Some(400));

    let mut brute: Vec<u32> = inserted.iter().map(|&v| v.abs_diff(query)).collect();
    brute.sort();

    let ann_min = ann_results
        .iter()
        .map(|(d, _)| *d)
        .min()
        .expect("ANN must return at least one result for a non-empty index");
    assert_eq!(
        ann_min, brute[0],
        "ANN closest distance across multi-layer graph must match brute-force"
    );
}

// ---------------------------------------------------------------------------
// Test - heuristic_produces_different_graph_than_greedy
//
// Confirms that HEURISTIC=true actually activates a different neighbor
// selection branch (add_node_heuristic vs add_node) and produces a
// structurally distinct zero_layer graph.
//
// The heuristic selection algorithm (select_neighbors_heuristic) applies a
// diversity criterion: candidate C is only added if it is closer to the
// new node than to any already-selected neighbor. For 1D sequential data
// this produces sparser, more "local" connections than greedy - for an
// interior node (e.g. 30) greedy picks up to M0 neighbors in both directions
// while heuristic quickly saturates with the two nearest and skips further
// nodes whose closest already-selected neighbor is nearer than the new node.
//
// The test compares zero_layer edge multisets (sorted node-pair strings from
// draw_model()[0].1). If the graphs were identical, HEURISTIC=true would be
// dead code and config tests with that flag would be false positives.
// ---------------------------------------------------------------------------
#[test]
fn heuristic_produces_different_graph_than_greedy() {
    let n = 50u32;

    let mut greedy = ApoNum::new();
    let mut heuristic = ApoNumHeuristic::new();

    for i in 0..n {
        greedy.insert(SimpleNumberRecord::create(i.to_string())); // safe: sequential, unique
        heuristic.insert(SimpleNumberRecord::create(i.to_string())); // safe: sequential, unique
    }

    // Collect edge sets as sorted (src, dst) string pairs - direction-agnostic.
    let edges_for = |edges: Vec<(String, String, f32)>| -> HashSet<(String, String)> {
        edges
            .into_iter()
            .map(|(a, b, _)| if a <= b { (a, b) } else { (b, a) })
            .collect()
    };

    let greedy_edges = edges_for(greedy.hnsw.draw_model()[0].1.clone());
    let heuristic_edges = edges_for(heuristic.hnsw.draw_model()[0].1.clone());

    assert_ne!(
        greedy_edges, heuristic_edges,
        "HEURISTIC=true must produce a different zero_layer graph than HEURISTIC=false \
         (if equal, add_node_heuristic is not diverging from add_node for this dataset)"
    );
}
