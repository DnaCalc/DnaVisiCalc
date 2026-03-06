//! Experimental dynamic-array implementation patterns.
//!
//! These modules are intentionally isolated from the production engine so we can
//! compare alternate designs without destabilizing core behavior.

pub mod array_graph;
pub mod spill_overlay;
pub mod spill_rewrite;
