

use tlsh2::{TlshDefault};
use std::str::FromStr;

pub trait FeatureType {
    type IdType;
    
    fn get_id(&self) -> &Self::IdType; // TODO: Maybe not needed?
}

#[derive(Clone)]
pub struct NumberFeature {
    pub id: u32,
}

impl FeatureType for NumberFeature {
    type IdType = u32;
    
    #[inline(always)]
    fn get_id(&self) -> &Self::IdType {
        &self.id
    }
}

impl NumberFeature {
    pub fn new(id: u32) -> Self {
        Self { id }
    }
}

pub struct HashFeature {
    pub id: TlshDefault,
}

impl FeatureType for HashFeature {
    type IdType = TlshDefault;
    fn get_id(&self) -> &Self::IdType {
        &self.id
    }
}

impl HashFeature {
    pub fn new(s: TlshDefault) -> Self {
        HashFeature { id: s }
    }
}


// Manual clone implementation
impl Clone for HashFeature {
    fn clone(&self) -> Self {
        let hash = self.id.hash();
        let hash_bytes = hash.as_slice(); 
    
        let tlsh_str = std::str::from_utf8(hash_bytes).expect("TLSH bytes not valid UTF-8");
        let tlsh_clone = TlshDefault::from_str(tlsh_str);

        HashFeature::new(tlsh_clone.unwrap())
    }
}


pub struct TlshHashFeature {
    pub id: TlshDefault,
    pub string: String,
}

impl FeatureType for TlshHashFeature {
    type IdType = TlshDefault;
    fn get_id(&self) -> &Self::IdType {
        &self.id
    }

    
}

impl TlshHashFeature {
    pub fn new(s: String) -> Self {
        let id = TlshDefault::from_str(&s).unwrap();
        TlshHashFeature { id, string: s }
    }
}


// Manual clone implementation
impl Clone for TlshHashFeature {
    fn clone(&self) -> Self {
        let hash = self.id.hash();
        let hash_bytes = hash.as_slice(); 
    
        let tlsh_str = std::str::from_utf8(hash_bytes).expect("TLSH bytes not valid UTF-8");
        TlshHashFeature::new(tlsh_str.to_string())
    }
}
