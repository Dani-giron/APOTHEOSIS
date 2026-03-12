use std::str::FromStr;
use tlsh2::TlshDefault;

/// The primary trait for any entity inserted into the Apotheosis Database.
/// Implement this trait on domain objects to define how they should be indexed
/// by the Radix exact-match tree and the HNSW proximity graph.
pub trait ApotheosisRecord {
    type MetricId: Clone; // The mathematical/hashing space representation used by HNSW. E.g., `tlsh2::TlshDefault` or `u32`.
    fn radix_key(&self) -> Vec<u8>; // The exact-match string identifier used in the Radix Tree.
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

impl<ID: Clone> ApotheosisRecord for SimpleRecord<ID> {
    type MetricId = ID;

    fn radix_key(&self) -> Vec<u8> {
        self.radix_key.clone().into_bytes()
    }

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

    fn radix_key(&self) -> Vec<u8> {
        self.internal_filename.clone().into_bytes()
    }

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

    fn radix_key(&self) -> Vec<u8> {
        self.hash.hash().to_vec()
    }

    fn search_id(&self) -> Self::MetricId {
        self.hash.clone() // Simplified dummy behavior for tests
    }

    fn get_attributes(&self) -> Vec<(String, String)> {
        vec![("version_names".to_string(), "".to_string())]
    }
}

#[derive(Debug)]
pub struct OpenWrt {
    pub version: String,
    pub binary: String,
    pub function_name: String,
}

pub struct OpenWrtRecord {
    pub hash: TlshDefault,
    pub data: Vec<OpenWrt>,
}

impl ApotheosisRecord for OpenWrtRecord {
    type MetricId = TlshDefault;

    fn radix_key(&self) -> Vec<u8> {
        self.hash.hash().to_vec()
    }

    fn search_id(&self) -> Self::MetricId {
        self.hash.clone()
    }
}
