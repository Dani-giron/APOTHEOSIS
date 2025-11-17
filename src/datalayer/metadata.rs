pub trait ApotheosisMetadata {
    fn get_attributes(&self) -> Vec<(String, String)>;
}

impl ApotheosisMetadata for () {
    fn get_attributes(&self) -> Vec<(String, String)> {
        vec![] 
    }
}

// -- Define your custom metadata structure here (examples below) --
pub struct FileMetadata {
    pub filename: String,
    pub file_path: String,
    pub version: String,
    pub size: u64,
    pub timestamp: u64,
}

impl ApotheosisMetadata for FileMetadata {
    fn get_attributes(&self) -> Vec<(String, String)> {
        vec![
            ("filename".to_string(), self.filename.clone()),
            ("file_path".to_string(), self.file_path.clone()),
            ("version".to_string(), self.version.clone()),
            ("size".to_string(), self.size.to_string()),
            ("timestamp".to_string(), self.timestamp.to_string()),
        ]
    }
}

#[derive(Clone)]
pub struct WinModuleMetadata {
    pub id: u32,
    pub file_version: String,
    pub internal_filename: String,
    pub product: String,
    pub company: String,
    pub os_id: u32,
}

impl ApotheosisMetadata for WinModuleMetadata {
    fn get_attributes(&self) -> Vec<(String, String)> {
        vec![
            ("id".to_string(), self.id.to_string()),
            ("file_version".to_string(), self.file_version.clone()),
            ("internal_filename".to_string(), self.internal_filename.clone()),
            ("product".to_string(), self.product.clone()),
            ("company".to_string(), self.company.clone()),
            ("os_id".to_string(), self.os_id.to_string()),
        ]
    }
}

pub struct BinaryMetadata {
    pub versions_names: Vec<(String, String)>,
}

impl ApotheosisMetadata for BinaryMetadata {
    fn get_attributes(&self) -> Vec<(String, String)> {
        vec![
            ("version_names".to_string(), "".to_string()),
        ]
    }
}