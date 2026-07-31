// TLSH correctness tests for APOTHEOSIS 2
//
// The rest of the suite runs on u32/NormalDistance, a degenerate 1-D metric
// space. These tests exercise the actual production domain: TLSH digests with
// TlshDistance. Digests are built deterministically in-test from seeded
// buffers, so no external dataset file is needed.

use apotheosis2::controllers::apotheosis::Apotheosis;
use apotheosis2::datalayer::algorithms::TlshDistance;
use apotheosis2::datalayer::record::SimpleTlshRecord;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use tlsh2::{TlshDefault, TlshDefaultBuilder};

type ApoTlsh = Apotheosis<SimpleTlshRecord, TlshDistance>;

// Deterministic TLSH digests: 1 KiB random buffers from a fixed seed. TLSH
// needs at least 50 bytes with enough entropy; 1 KiB of PRNG output always
// produces a valid digest.
fn build_digests(count: usize) -> Vec<TlshDefault> {
    let mut rng = StdRng::seed_from_u64(7);
    (0..count)
        .map(|_| {
            let buf: Vec<u8> = (0..1024).map(|_| rng.r#gen::<u8>()).collect();
            TlshDefaultBuilder::build_from(&buf).expect("1 KiB random buffer must produce a TLSH")
        })
        .collect()
}

fn hash_string(digest: &TlshDefault) -> String {
    String::from_utf8(digest.hash().to_vec()).expect("TLSH hash is ASCII hex")
}

// ---------------------------------------------------------------------------
// Exact match through the radix fast-path with real TLSH keys.
// ---------------------------------------------------------------------------
#[test]
fn tlsh_exact_match_returns_inserted_record() {
    let digests = build_digests(30);
    let mut idx = ApoTlsh::new();
    for d in &digests {
        idx.insert(SimpleTlshRecord::create(hash_string(d)));
    }

    for d in &digests {
        let results = idx.search(d, 1, None);
        assert!(!results.is_empty(), "exact TLSH search returned nothing");
        assert_eq!(results[0].0, 0, "exact TLSH match must have distance 0");
        assert_eq!(
            results[0].1.radix_key,
            hash_string(d),
            "index desync: exact TLSH search returned another record"
        );
    }
}

// ---------------------------------------------------------------------------
// ANN path on TLSH space agrees with brute force on the nearest neighbor.
// ---------------------------------------------------------------------------
#[test]
fn tlsh_ann_nearest_matches_brute_force() {
    let digests = build_digests(60);
    let (dataset, queries) = digests.split_at(50);

    let mut idx = ApoTlsh::new();
    for d in dataset {
        idx.insert(SimpleTlshRecord::create(hash_string(d)));
    }

    let dist = TlshDistance;
    for q in queries {
        let results = idx.search(q, 5, Some(100));
        assert!(!results.is_empty(), "TLSH ANN search returned nothing");

        use apotheosis2::datalayer::algorithms::DistanceAlgorithm;
        let brute_min = dataset
            .iter()
            .map(|d| dist.calculate_distance(d, q))
            .min()
            .unwrap();
        assert_eq!(
            results[0].0, brute_min,
            "TLSH ANN nearest must match brute-force nearest"
        );
    }
}

// ---------------------------------------------------------------------------
// dump/load roundtrip preserves TLSH search results (serde path for TLSH).
// ---------------------------------------------------------------------------
#[test]
fn tlsh_dump_load_roundtrip_preserves_results() {
    use std::fs;

    let path = "target/test_tlsh_roundtrip.bin";
    let digests = build_digests(20);
    let mut idx = ApoTlsh::new();
    for d in &digests {
        idx.insert(SimpleTlshRecord::create(hash_string(d)));
    }

    let query = &digests[3];
    let before: Vec<u32> = idx
        .search(query, 3, Some(50))
        .iter()
        .map(|(d, _)| *d)
        .collect();

    idx.dump(path).expect("dump() must not fail");
    let loaded = ApoTlsh::load(path).expect("load() must not fail");
    let after: Vec<u32> = loaded
        .search(query, 3, Some(50))
        .iter()
        .map(|(d, _)| *d)
        .collect();

    fs::remove_file(path).ok();
    assert_eq!(before, after, "TLSH results must survive dump/load");
}
