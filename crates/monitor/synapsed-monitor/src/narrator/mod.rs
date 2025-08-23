//! Natural language narration of events

mod event_narrator;
mod pattern_detector;
mod templates;

pub use event_narrator::{EventNarrator, Narrative, NarrativeStyle};
pub use pattern_detector::{PatternDetector, DetectedPattern};
pub use templates::{NarrativeTemplate, TemplateEngine};