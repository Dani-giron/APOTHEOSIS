pub trait ApotheosisMetadata {
    fn get_attributes(&self) -> Vec<(String, String)>;
}

impl ApotheosisMetadata for () {
    fn get_attributes(&self) -> Vec<(String, String)> {
        vec![] 
    }
}

// -- Define your custom metadata structure here --
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
