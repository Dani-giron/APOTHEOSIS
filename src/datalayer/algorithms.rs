use tlsh2::TlshDefault;

pub trait DistanceAlgorithm<ID> {
    /// Stable identifier for this distance function, written to the model
    /// header by `dump()` and checked by `load()`. It must be unique per
    /// distance type and must not change once models have been persisted,
    /// otherwise a file built with one distance would load into a type that
    /// interprets it with a different one, returning wrong results silently.
    const TYPE_NAME: &'static str;

    fn calculate_distance(&self, a: &ID, b: &ID) -> u32;
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct NormalDistance;
impl DistanceAlgorithm<u32> for NormalDistance {
    const TYPE_NAME: &'static str = "NormalDistance";

    fn calculate_distance(&self, a: &u32, b: &u32) -> u32 {
        a.abs_diff(*b)
    }
}

#[derive(Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct TlshDistance;
impl DistanceAlgorithm<TlshDefault> for TlshDistance {
    const TYPE_NAME: &'static str = "TlshDistance";

    fn calculate_distance(&self, a: &TlshDefault, b: &TlshDefault) -> u32 {
        let diff = a.diff(&b, true);
        diff as u32
    }
}
