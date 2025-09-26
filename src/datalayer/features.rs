

// TODO: Rethink this. Maybe not the best approach

use tlsh2::{TlshDefault};
use std::str::FromStr;

pub trait FeatureType {
    type IdType;
     
    fn create(s: String) -> Self;
    fn get_id(&self) -> &Self::IdType;
    fn get_radix_key(&self) -> &String;
}

#[derive(Clone)]
pub struct NumberFeature {
    pub id: u32,
    pub radix_key: String,
    
}

impl FeatureType for NumberFeature {
    type IdType = u32;

    fn create(s: String) -> Self {
        Self { id: s.parse::<u32>().unwrap(), radix_key: s }
    }
    
    #[inline(always)]
    fn get_id(&self) -> &Self::IdType {
        &self.id
    }

    #[inline(always)]
    fn get_radix_key(&self) -> &String{
        &self.radix_key
    }
}

pub struct TlshHashFeature {
    pub id: TlshDefault,
    pub radix_key: String,
}

impl FeatureType for TlshHashFeature {
    type IdType = TlshDefault;

    fn create(s: String) -> Self {
        let id = TlshDefault::from_str(&s).unwrap();
        TlshHashFeature { id, radix_key: s }
    }

    fn get_id(&self) -> &Self::IdType {
        &self.id
    }

    fn get_radix_key(&self) -> &String{
        &self.radix_key
    }
}

// Manual clone implementation
impl Clone for TlshHashFeature {
    fn clone(&self) -> Self {
        let hash = self.id.hash();
        let hash_bytes = hash.as_slice(); 
    
        let tlsh_str = std::str::from_utf8(hash_bytes).expect("TLSH bytes not valid UTF-8");
        TlshHashFeature::create(tlsh_str.to_string())
    }
}
