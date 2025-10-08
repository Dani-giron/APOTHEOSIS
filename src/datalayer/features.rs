use tlsh2::{TlshDefault};
use std::str::FromStr;

pub trait FeatureType {
    type IdType;
    
    fn create(s: String) -> Self;
    fn get_id(&self) -> &Self::IdType;
    fn get_radix_key(&self) -> &String;
}

pub struct Feature<ID> {
    pub id: ID,
    pub radix_key: String,
}

impl<ID> Feature<ID> {
    pub fn new(id: ID, radix_key: String) -> Self {
        Self { id, radix_key, }
    }
}

impl<ID> Feature<ID> {
    pub fn simple(id: ID, radix_key: String) -> Self {
        Self {
            id,
            radix_key,
        }
    }
}

pub type NumberFeature = Feature<u32>;
pub type TlshHashFeature = Feature<TlshDefault>;

impl FeatureType for NumberFeature {
    type IdType = u32;
    
    fn create(s: String) -> Self {
        let id = s.parse::<u32>().unwrap();
        Self::simple(id, s)
    }
    
    #[inline(always)]
    fn get_id(&self) -> &Self::IdType {
        &self.id
    }
    
    #[inline(always)]
    fn get_radix_key(&self) -> &String {
        &self.radix_key
    }
}

impl FeatureType for TlshHashFeature {
    type IdType = TlshDefault;
    
    fn create(s: String) -> Self {
        let id = TlshDefault::from_str(&s).unwrap();
        Self { id: id, radix_key: s }
    }
    
    #[inline(always)]
    fn get_id(&self) -> &Self::IdType {
        &self.id
    }
    
    #[inline(always)]
    fn get_radix_key(&self) -> &String {
        &self.radix_key
    }
}