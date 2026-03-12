use tlsh2::TlshDefault;

pub trait DistanceAlgorithm<ID> {
    fn calculate_distance(&self, a: &ID, b: &ID) -> u32;
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct NormalDistance;
impl DistanceAlgorithm<u32> for NormalDistance {
    fn calculate_distance(&self, a: &u32, b: &u32) -> u32 {
        a.abs_diff(*b)
    }
}

#[derive(Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct TlshDistance;
impl DistanceAlgorithm<TlshDefault> for TlshDistance {
    fn calculate_distance(&self, a: &TlshDefault, b: &TlshDefault) -> u32 {
        let diff = a.diff(&b, true);
        diff as u32
    }
}
