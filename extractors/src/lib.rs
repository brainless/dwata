//! Extractors Crate
//!
//! This crate provides various extraction implementations for extracting structured entities
//! from email data. It is designed to be reusable across different projects.
//!
//! # Architecture
//!
//! - **Types**: Entity types and traits are defined in the `shared-types` crate
//! - **Implementations**: Concrete extractors are implemented in this crate
//!
//! # Available Extractors
//!
//! - `AttachmentParserExtractor`: Extracts entities from structured attachments (ICS, VCF)
//!
//! # Example
//!
//! ```rust,ignore
//! use extractors::{AttachmentParserExtractor, IcsParserConfig};
//! use shared_types::{Extractor, ExtractionInput};
//!
//! let extractor = AttachmentParserExtractor::with_defaults();
//! let results = extractor.extract(&input)?;
//! ```

pub mod attachment_parser;

// Re-export commonly used types
pub use attachment_parser::{AttachmentParserExtractor, IcsParserConfig, TimezoneHandling};

// Re-export the Extractor trait from shared-types for convenience
pub use shared_types::Extractor;
