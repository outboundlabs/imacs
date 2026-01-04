//! # Espresso Logic Minimizer
//!
//! A Rust implementation of the Espresso heuristic logic minimizer.
//!
//! Espresso is a Boolean minimization algorithm that takes a two-level representation
//! of a Boolean function and produces a minimal equivalent representation using
//! heuristic methods.
//!
//! ## Overview
//!
//! The algorithm operates on "cubes" which represent product terms in Sum-of-Products
//! form. Each cube is a vector where each variable can be:
//! - `0` (variable must be false)
//! - `1` (variable must be true)
//! - `-` (don't care - variable can be either)
//!
//! ## Example
//!
//! ```ignore
//! use imacs::completeness::espresso::{Cover, Cube, espresso};
//!
//! // Create cubes for the function: AB' + A'B + AB
//! let mut on_set = Cover::new(2, 1);
//! on_set.add(Cube::from_str("10", "1").unwrap());  // AB'
//! on_set.add(Cube::from_str("01", "1").unwrap());  // A'B
//! on_set.add(Cube::from_str("11", "1").unwrap());  // AB
//!
//! let dc_set = Cover::new(2, 1);  // No don't cares
//!
//! let minimized = espresso(&on_set, &dc_set);
//! // Result should be: A + B (two terms instead of three)
//! ```

pub mod cover;
pub mod cube;
pub mod error;
pub mod minimize;
pub mod pla;

pub use cover::Cover;
pub use cube::{Cube, CubeValue};
pub use error::EspressoError;
pub use minimize::{espresso, EspressoOptions};
pub use pla::Pla;

/// Result type for espresso operations
pub type Result<T> = std::result::Result<T, EspressoError>;
