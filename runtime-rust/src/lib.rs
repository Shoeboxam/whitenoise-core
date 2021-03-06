//! The Whitenoise rust runtime is an execution engine for evaluating differentially private analyses.
//! 
//! The runtime contains implementations of basic data transformations and aggregations, 
//! statistics, and privatizing mechanisms. These functions are combined in the 
//! Whitenoise validator to create more complex differentially private analyses. 

extern crate whitenoise_validator;

pub use whitenoise_validator::proto;
use whitenoise_validator::errors::*;

// trait which holds `display_chain`

pub mod base;
pub mod utilities;
pub mod components;
pub mod ffi;

extern crate libc;

/// Evaluate an analysis and release the differentially private results.
pub fn release(
    request: &proto::RequestRelease
) -> Result<proto::Release> {
    base::execute_graph(
        request.analysis.as_ref()
            .ok_or_else(|| Error::from("analysis must be defined"))?,
        request.release.as_ref()
            .ok_or_else(|| Error::from("release must be defined"))?)
}
