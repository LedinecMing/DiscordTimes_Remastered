//! lit2png — Discord Times (2003) LIT/UGS graphics → PNG converter.
//!
//! Format reference: notes/RUST_CONVERTER_GUIDE.md.

pub mod error;
pub mod lit;
pub mod pngout;
pub mod ugs;

pub use error::{LitError, Result};
