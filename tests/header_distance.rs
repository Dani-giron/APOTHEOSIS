// dump()/load() must reject a model built with one distance and loaded into a
// type using a different distance. bincode does not depend on the distance
// type, so without this check the graph would deserialize and every later
// search would compute distances with the wrong function, silently.

use apotheosis2::controllers::apotheosis::Apotheosis;
use apotheosis2::datalayer::algorithms::{DistanceAlgorithm, NormalDistance};
use apotheosis2::datalayer::record::SimpleNumberRecord;

// A second distance over the same metric space (u32) as NormalDistance, so the
// only difference between the two models is the distance type. Everything else
// (record type, MetricId, M, M0, EF, HEURISTIC) is identical.
#[derive(Default, serde::Serialize, serde::Deserialize)]
struct InvertedDistance;
impl DistanceAlgorithm<u32> for InvertedDistance {
    const TYPE_NAME: &'static str = "InvertedDistance";

    fn calculate_distance(&self, a: &u32, b: &u32) -> u32 {
        u32::MAX - a.abs_diff(*b)
    }
}

type ApoNormal = Apotheosis<SimpleNumberRecord, NormalDistance>;
type ApoInverted = Apotheosis<SimpleNumberRecord, InvertedDistance>;

#[test]
fn load_rejects_distance_mismatch() {
    let path = "target/test_header_distance_mismatch.bin";
    let _ = std::fs::remove_file(path);

    let mut idx = ApoNormal::new();
    idx.insert(SimpleNumberRecord::create("1".to_string()));
    idx.dump(path).expect("dump() must not fail");

    // Same M/M0/EF/HEURISTIC and record type, only the distance differs.
    let result = ApoInverted::load(path);

    let _ = std::fs::remove_file(path);

    assert!(
        result.is_err(),
        "load() must reject a model built with a different distance"
    );
}

#[test]
fn load_accepts_matching_distance() {
    let path = "target/test_header_distance_match.bin";
    let _ = std::fs::remove_file(path);

    let mut idx = ApoNormal::new();
    idx.insert(SimpleNumberRecord::create("1".to_string()));
    idx.dump(path).expect("dump() must not fail");

    let loaded = ApoNormal::load(path);

    let _ = std::fs::remove_file(path);

    assert!(
        loaded.is_ok(),
        "load() with the same distance must succeed: {:?}",
        loaded.err()
    );
}
