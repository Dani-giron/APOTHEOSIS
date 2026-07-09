use apotheosis2::controllers::apotheosis::Apotheosis;
use apotheosis2::datalayer::algorithms::NormalDistance;
use apotheosis2::datalayer::record::SimpleNumberRecord;

type ApoNum = Apotheosis<SimpleNumberRecord, NormalDistance>;

// search() over an empty index must return an empty Vec without panicking.
// Before the fix (issue #8), knn_search() accessed zero_layer[0] without
// checking whether the slice was empty, causing a panic.
#[test]
fn search_on_empty_index_returns_empty_vec() {
    let idx = ApoNum::new();
    let results = idx.search(&0u32, 1, None);
    assert!(
        results.is_empty(),
        "search() over an empty index must return an empty Vec"
    );
}
