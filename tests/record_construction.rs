// create() constructors build records from untrusted strings (e.g. a line
// read from a dataset file). They must return Err on malformed input instead
// of panicking, so a single bad entry does not abort a whole ingestion run.

use apotheosis2::datalayer::record::{GenericJsonRecord, SimpleNumberRecord, SimpleTlshRecord};

#[test]
fn number_record_rejects_non_numeric_input() {
    let result = SimpleNumberRecord::create("not-a-number".to_string());
    assert!(result.is_err(), "malformed number input must return Err");

    let msg = result.err().unwrap().to_string();
    assert!(
        msg.contains("not-a-number"),
        "error message must include the offending input, got: {msg}"
    );
}

#[test]
fn number_record_accepts_valid_input() {
    let result = SimpleNumberRecord::create("42".to_string());
    assert!(result.is_ok());
    assert_eq!(result.unwrap().id, 42);
}

#[test]
fn tlsh_record_rejects_malformed_hash() {
    let result = SimpleTlshRecord::create("not-a-tlsh-hash".to_string());
    assert!(result.is_err(), "malformed TLSH hash must return Err");

    let msg = result.err().unwrap().to_string();
    assert!(
        msg.contains("not-a-tlsh-hash"),
        "error message must include the offending input, got: {msg}"
    );
}

#[test]
fn generic_json_record_rejects_malformed_hash() {
    let result = GenericJsonRecord::create(
        "not-a-tlsh-hash".to_string(),
        serde_json::json!({"key": "value"}),
    );
    assert!(result.is_err(), "malformed TLSH hash must return Err");
}
