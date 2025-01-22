//! Search utilities for Raves.

pub mod details;
pub mod modifiers;
pub mod query;
pub mod sort;

/// `modifier1 AND modifier2`
pub struct AndBlock();

/// `modifier1 OR modifier2`
pub struct OrBlock;

/// `NOT(modifier)`
pub struct NotBlock;

/// `BEFORE(datetime)`
pub struct BeforeBlock;

/// `DURING(datetime)`
pub struct DuringBlock;

/// `AFTER(datetime)`
pub struct AfterBlock;

pub trait SearchElement {}
