use std::str::FromStr;
use tlsh2::{TlshDefault};

/// Trait for Metric types (like TLSH or numerical IDs) to specify if they natively map to a Radix Tree exact-match key.
pub trait RadixKeyMapping {
    fn to_radix_key(&self) -> Option<Vec<u8>> {
        None
    }
}

impl RadixKeyMapping for TlshDefault {
    fn to_radix_key(&self) -> Option<Vec<u8>> {
        Some(self.hash().to_vec())
    }
}

impl RadixKeyMapping for u32 {
    fn to_radix_key(&self) -> Option<Vec<u8>> {
        Some(self.to_string().into_bytes())
    }
}

/// The primary trait for any entity inserted into the Apotheosis Database.
/// Implement this trait on domain objects to define how they should be indexed
/// by the Radix exact-match tree and the HNSW proximity graph.
pub trait ApotheosisRecord {
    type MetricId: Clone + RadixKeyMapping; // The mathematical/hashing space representation used by HNSW. E.g., `tlsh2::TlshDefault` or `u32`.
    fn search_id(&self) -> Self::MetricId; // The metric representation (e.g., hash) used for similarity calculation in the HNSW graph.

    fn get_attributes(&self) -> Vec<(String, String)> {
        // Optional metadata key-value pairs associated with the ApotheosisRecord. These are exported during model visualization (GEXF).
        vec![]
    }
}

// Simple record types (Nodes with no metadata / Dummy records)

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct SimpleRecord<ID> {
    pub id: ID,
    pub radix_key: String,
}

impl<ID: Clone + RadixKeyMapping> ApotheosisRecord for SimpleRecord<ID> {
    type MetricId = ID;

    fn search_id(&self) -> Self::MetricId {
        self.id.clone()
    }
}

pub type SimpleNumberRecord = SimpleRecord<u32>;
pub type SimpleTlshRecord = SimpleRecord<TlshDefault>;

impl SimpleNumberRecord {
    pub fn create(s: String) -> Self {
        let id = s.parse::<u32>().unwrap();
        Self { id, radix_key: s }
    }
}

impl SimpleTlshRecord {
    pub fn create(s: String) -> Self {
        let id = TlshDefault::from_str(&s).unwrap();
        Self { id, radix_key: s }
    }
}

// TLSH node that accepts any JSON as metadata
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct GenericJsonRecord {
    pub id: TlshDefault,
    pub radix_key: String,
    pub metadata: serde_json::Value,
}

impl ApotheosisRecord for GenericJsonRecord {
    type MetricId = TlshDefault;

    fn search_id(&self) -> Self::MetricId {
        self.id.clone()
    }

    fn get_attributes(&self) -> Vec<(String, String)> {
        if let Some(obj) = self.metadata.as_object() {
            obj.iter()
                .map(|(k, v)| {
                    let val_str = match v {
                        serde_json::Value::String(s) => s.clone(),
                        _ => v.to_string(),
                    };
                    (k.clone(), val_str)
                })
                .collect()
        } else {
            vec![("metadata".to_string(), self.metadata.to_string())]
        }
    }
}

impl GenericJsonRecord {
    pub fn create(s: String, metadata: serde_json::Value) -> Self {
        let id = TlshDefault::from_str(&s).unwrap();
        Self {
            id,
            radix_key: s,
            metadata,
        }
    }
}

// Example/Existing Implementations

pub struct FileRecord {
    pub hash: TlshDefault,
    pub filename: String,
    pub file_path: String,
    pub version: String,
    pub size: u64,
    pub timestamp: u64,
}

#[derive(Clone)]
pub struct WinModuleRecord {
    pub hash: TlshDefault,
    pub id: u32,
    pub file_version: String,
    pub internal_filename: String,
    pub product: String,
    pub company: String,
    pub os_id: u32,
}

impl ApotheosisRecord for WinModuleRecord {
    type MetricId = TlshDefault;

    fn search_id(&self) -> Self::MetricId {
        self.hash.clone()
    }

    fn get_attributes(&self) -> Vec<(String, String)> {
        vec![
            ("id".to_string(), self.id.to_string()),
            ("file_version".to_string(), self.file_version.clone()),
            (
                "internal_filename".to_string(),
                self.internal_filename.clone(),
            ),
            ("product".to_string(), self.product.clone()),
            ("company".to_string(), self.company.clone()),
            ("os_id".to_string(), self.os_id.to_string()),
        ]
    }
}

pub struct BinaryRecord {
    pub hash: TlshDefault,
    pub versions_names: Vec<(String, String)>,
}

impl ApotheosisRecord for BinaryRecord {
    type MetricId = TlshDefault;

    fn search_id(&self) -> Self::MetricId {
        self.hash.clone() // Simplified dummy behavior for tests
    }

    fn get_attributes(&self) -> Vec<(String, String)> {
        vec![("version_names".to_string(), "".to_string())]
    }
}
