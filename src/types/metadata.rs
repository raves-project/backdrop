use serde::{Deserialize, Serialize};

pub trait Metadata {}

pub type FileSize = u64;
pub type FileName = String;
pub type CreatedDate = u64;
pub type ModifiedDate = u64;
pub type FirstSeenDate = u64;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Resolution {
    pub x: u32,
    pub y: u32,
}
