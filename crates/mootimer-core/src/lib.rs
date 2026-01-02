//! MooTimer Core Library
//!
//! This crate contains the core data models, storage logic, and utilities
//! for the MooTimer application.

pub mod error;
pub mod git;
pub mod models;
pub mod storage;

pub use error::{Error, Result};
