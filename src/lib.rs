//! # rs-trafilatura
//!
//! Rust port of trafilatura - a web content extraction library.
//!
//! This library extracts clean, readable content from web pages by stripping
//! navigation, advertisements, and boilerplate while preserving meaningful
//! text, metadata, and document structure.
//!
//! ## Quick Start
//!
//! ```rust
//! use rs_trafilatura::{extract, Options};
//!
//! let html = r#"<html><head><title>My Article</title></head>
//! <body><article><p>Main content here.</p></article></body></html>"#;
//!
//! let result = extract(html)?;
//! println!("Title: {:?}", result.metadata.title);
//! println!("Content: {}", result.content_text);
//! # Ok::<(), rs_trafilatura::Error>(())
//! ```
//!
//! ## Features
//!
//! - **Content Extraction**: Identifies and extracts the main article content
//! - **Page Type Classification**: XGBoost classifier detects 7 page types
//!   (article, forum, product, collection, listing, documentation, service)
//! - **Per-Type Extraction Profiles**: Type-specific boilerplate removal,
//!   content selectors, and extraction strategies
//! - **Extraction Quality Predictor**: ML confidence score (0.0-1.0) predicting
//!   extraction F1 — enables hybrid pipelines with LLM fallback for low-confidence pages
//! - **Metadata Extraction**: Title, author, date, language, sitename, and more
//!   from JSON-LD, Open Graph, Dublin Core, and HTML meta tags
//! - **Markdown Output**: GitHub Flavored Markdown with headings, lists, tables, code blocks
//! - **Configurable**: Options to tune precision/recall tradeoff
//!
//! ## Accuracy
//!
//! Achieves F1 0.859 on a 1,497-page multi-type benchmark (WCXB), outperforming
//! Trafilatura (0.792) and neural approaches MinerU-HTML (0.827) and ReaderLM-v2 (0.741).
//! F1 0.893 on a 511-page held-out test set confirms generalization.

mod error;
mod extract;
mod options;
mod patterns;
mod result;

/// Page type classification (URL heuristics, HTML signals, ML classifier).
pub mod page_type;

/// F-Score calculation for accuracy benchmarking.
pub mod scoring;

/// Markdown processing utilities (escaping, table conversion).
pub mod markdown;

/// Character encoding detection and transcoding.
pub mod encoding;

/// Integration with the [spider](https://crates.io/crates/spider) web crawler.
///
/// Enable with the `spider` feature flag in your `Cargo.toml`:
/// ```toml
/// rs-trafilatura = { version = "0.2", features = ["spider"] }
/// ```
#[cfg(feature = "spider")]
pub mod spider_integration;

// Internal modules — not part of the public API
#[allow(dead_code)]
pub(crate) mod dom;
pub(crate) mod etree;
#[allow(dead_code)]
pub(crate) mod lru;
#[allow(dead_code)]
pub(crate) mod selector;
#[allow(dead_code)]
pub(crate) mod html_processing;
#[allow(dead_code)]
pub(crate) mod extractor;
#[allow(dead_code)]
pub(crate) mod metadata;
#[allow(dead_code)]
pub(crate) mod url_utils;
#[allow(dead_code)]
pub(crate) mod link_density;

// Public API - re-exports
pub use error::{Error, Result};
pub use options::Options;
pub use result::{BaselineResult, ExtractResult, ImageData, Metadata};

/// Extracts main content from an HTML document using default options.
///
/// # Arguments
///
/// * `html` - The HTML document as a string slice
///
/// # Returns
///
/// Returns `Ok(ExtractResult)` on success, containing the extracted content
/// and metadata. Returns an `Error` if extraction fails completely.
///
/// # Example
///
/// ```rust
/// use rs_trafilatura::extract;
///
/// let html = "<html><body><article>Content</article></body></html>";
/// let result = extract(html)?;
/// println!("{}", result.content_text);
/// # Ok::<(), rs_trafilatura::Error>(())
/// ```
#[allow(clippy::missing_errors_doc)]
pub fn extract(html: &str) -> Result<ExtractResult> {
    extract_with_options(html, &Options::default())
}

/// Extracts main content from an HTML document with custom options.
///
/// # Arguments
///
/// * `html` - The HTML document as a string slice
/// * `options` - Configuration options for extraction behavior
///
/// # Returns
///
/// Returns `Ok(ExtractResult)` on success, containing the extracted content
/// and metadata. Returns an `Error` if extraction fails completely.
///
/// # Example
///
/// ```rust
/// use rs_trafilatura::{extract_with_options, Options};
///
/// let html = "<html><body><article>Content</article></body></html>";
/// let options = Options {
///     include_tables: true,
///     favor_precision: true,
///     ..Options::default()
/// };
/// let result = extract_with_options(html, &options)?;
/// # Ok::<(), rs_trafilatura::Error>(())
/// ```
#[allow(clippy::missing_errors_doc)]
pub fn extract_with_options(html: &str, options: &Options) -> Result<ExtractResult> {
    extract::extract_content(html, options)
}

/// Low-level extraction helper matching trafilatura's `bare_extraction` role.
///
/// Rust already returns structured data from `extract_with_options`, so this is
/// an explicit compatibility alias for callers porting from Python.
#[allow(clippy::missing_errors_doc)]
pub fn bare_extraction(html: &str, options: &Options) -> Result<ExtractResult> {
    extract_with_options(html, options)
}

/// Extract main content and metadata using default options.
///
/// The Rust result always carries metadata, so this is a compatibility alias
/// for trafilatura's `extract_with_metadata` helper.
#[allow(clippy::missing_errors_doc)]
pub fn extract_with_metadata(html: &str) -> Result<ExtractResult> {
    extract(html)
}

/// Extract document metadata without running full content extraction.
#[must_use]
pub fn extract_metadata(html: &str, options: &Options) -> Metadata {
    let document = dom_query::Document::from(html);
    metadata::extract_metadata(&document, options)
}

/// Parse HTML into a DOM document.
///
/// This is the Rust equivalent of trafilatura's local `load_html` helper. The
/// underlying parser is forgiving and returns a document for malformed HTML.
#[must_use]
pub fn load_html(html: &str) -> dom_query::Document {
    dom_query::Document::from(html)
}

/// Parse HTML bytes into a DOM document with charset detection.
#[must_use]
pub fn load_html_bytes(html: &[u8]) -> dom_query::Document {
    let html_str = encoding::transcode_to_utf8(html);
    load_html(&html_str)
}

/// Run baseline extraction targeting JSON-LD, article tags, paragraphs, then body text.
#[must_use]
pub fn baseline(html: &str) -> BaselineResult {
    let document = load_html(html);
    let (body_doc, text) = extractor::fallback::baseline(&document);
    let body = body_doc.select("body");
    let body_html = if body.length() > 0 {
        dom::outer_html(&body).to_string()
    } else {
        String::new()
    };
    let len = text.chars().count();

    BaselineResult {
        body_html,
        text,
        len,
    }
}

/// Run basic HTML-to-text conversion.
///
/// When `clean` is true, common boilerplate and non-text elements are removed
/// before whitespace-normalized text extraction.
#[must_use]
pub fn html2txt(content: &str, clean: bool) -> String {
    let document = load_html(content);

    if clean {
        let nodes = document
            .select("aside, footer, nav, header, script, style, noscript")
            .nodes()
            .to_vec();
        for node in nodes.into_iter().rev() {
            etree::remove(&dom_query::Selection::from(node), false);
        }
    }

    let body = document.select("body");
    if body.length() == 0 {
        return String::new();
    }

    dom::text_content(&body)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Extracts main content from HTML bytes with automatic encoding detection.
///
/// This function accepts HTML as raw bytes, detects the character encoding
/// from meta tags, and converts to UTF-8 before extraction.
///
/// # Arguments
///
/// * `html` - The HTML document as raw bytes
///
/// # Returns
///
/// Returns `Ok(ExtractResult)` on success, containing the extracted content
/// and metadata. Returns an `Error` if extraction fails completely.
///
/// # Character Encoding
///
/// The function detects encoding from:
/// - `<meta charset="...">`
/// - `<meta http-equiv="Content-Type" content="...; charset=...">`
/// - Defaults to UTF-8 if no declaration found
///
/// Invalid characters are replaced with � (Unicode replacement character)
/// rather than causing errors.
///
/// # Example
///
/// ```rust
/// use rs_trafilatura::extract_bytes;
///
/// // ISO-8859-1 encoded HTML with charset declaration
/// let html = b"<html><head><meta charset=\"ISO-8859-1\"></head><body><article>Caf\xE9</article></body></html>";
/// let result = extract_bytes(html)?;
/// assert!(result.content_text.contains("Café"));
/// # Ok::<(), rs_trafilatura::Error>(())
/// ```
#[allow(clippy::missing_errors_doc)]
pub fn extract_bytes(html: &[u8]) -> Result<ExtractResult> {
    let html_str = encoding::transcode_to_utf8(html);
    extract(&html_str)
}

/// Extracts main content from HTML bytes with custom options and automatic encoding detection.
///
/// This combines the functionality of `extract_bytes` and `extract_with_options`,
/// accepting raw bytes and custom extraction options.
///
/// # Arguments
///
/// * `html` - The HTML document as raw bytes
/// * `options` - Configuration options for extraction behavior
///
/// # Returns
///
/// Returns `Ok(ExtractResult)` on success, containing the extracted content
/// and metadata. Returns an `Error` if extraction fails completely.
///
/// # Example
///
/// ```rust
/// use rs_trafilatura::{extract_bytes_with_options, Options};
///
/// // Windows-1252 encoded HTML
/// let html = b"<html><head><meta charset=\"windows-1252\"></head><body><article>Content</article></body></html>";
/// let options = Options {
///     include_tables: true,
///     favor_precision: true,
///     ..Options::default()
/// };
/// let result = extract_bytes_with_options(html, &options)?;
/// # Ok::<(), rs_trafilatura::Error>(())
/// ```
#[allow(clippy::missing_errors_doc)]
pub fn extract_bytes_with_options(html: &[u8], options: &Options) -> Result<ExtractResult> {
    let html_str = encoding::transcode_to_utf8(html);
    extract_with_options(&html_str, options)
}
