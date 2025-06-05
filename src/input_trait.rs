use std::fmt::Debug;

pub trait SimInput: Default + Clone + Debug {
    /// reduced representation of the input for storage and transmission
    type Bytes: Sized
        + Copy
        + Default
        + PartialEq
        + Eq
        + std::fmt::Debug
        + serde::Serialize
        + serde::de::DeserializeOwned;
    /// returns a fixed sized byte representation of the input tick
    fn to_bytes(&self) -> Self::Bytes;
    /// returns Self from a fixed sized byte representation of the input tick
    fn from_bytes(bytes: Self::Bytes) -> Self;
}

pub trait TestInputBytes: SimInput {
    /// returns a fixed sized byte representation of the input tick
    fn new_test_simple(x: u32) -> Self::Bytes;
}
