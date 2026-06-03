//! Sample-template generation for assistant-assisted task creation (Module B1).
//!
//! - [`generator`]: the public [`generator::generate_sample`] entrypoint, its
//!   [`generator::SampleLlm`] trait, and the rendered [`generator::SampleTemplate`]
//!   data model.
//! - [`presets`]: deterministic, hand-curated fallback templates.
//! - [`validate`]: spec-conformance checking via [`validate::against_spec`].
//! - [`llm`]: the production [`llm::LlmClientSampleLlm`] (DeepSeek-backed B2).

pub mod generator;
pub mod llm;
pub mod presets;
pub mod validate;

pub use generator::{
    generate_sample, EnumOption, GenerateError, LlmFailure, SampleField, SampleLlm, SampleSource,
    SampleTemplate,
};
pub use llm::LlmClientSampleLlm;
