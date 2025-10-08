use tlsh2::TlshDefault;

pub trait DistanceAlgorithm<ID> {
    fn calculate_distance(a: &ID, b: &ID) -> u32;
}

pub struct NormalDistance;
impl DistanceAlgorithm<u32> for NormalDistance {
    fn calculate_distance(a: &u32, b: &u32) -> u32 {
        a.abs_diff(*b)
    }
}

#[derive(Clone)]
pub struct TlshDistance;
impl DistanceAlgorithm<TlshDefault> for TlshDistance {
    fn calculate_distance(a: &TlshDefault, b: &TlshDefault) -> u32 {
        let diff = a.diff(&b, true);
        diff as u32
    }
}
