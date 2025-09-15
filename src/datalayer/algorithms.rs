use crate::datalayer::features::{FeatureType, NumberFeature, HashFeature};
use tlsh2::TlshDefault;



pub trait DistanceAlgorithm<F>
    where 
    F: FeatureType,
{

    fn calculate_distance(&self, a: &F::IdType, b: &F::IdType) -> u32;
}

pub struct NormalDistance;
impl DistanceAlgorithm<NumberFeature> for NormalDistance {
    
    fn calculate_distance(&self, a: &u32, b: &u32) -> u32 {
        a.abs_diff(*b)
    }

}

#[derive(Clone)]
pub struct TLSHDistance;
impl DistanceAlgorithm<HashFeature> for TLSHDistance {
    fn calculate_distance(&self, a: &TlshDefault, b: &TlshDefault) -> u32 {

        let diff = a.diff(&b, true);
        diff as u32
    }

}