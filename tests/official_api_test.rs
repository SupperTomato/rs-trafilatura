use rs_trafilatura::{
    bare_extraction, baseline, extract_metadata, extract_with_metadata, html2txt, load_html,
    load_html_bytes, Options,
};

#[test]
fn official_compat_helpers_extract_content_and_metadata() {
    let html = r#"
        <html>
          <head>
            <title>Compatibility Article</title>
            <meta name="author" content="Ada Lovelace">
            <link rel="canonical" href="https://example.com/article">
          </head>
          <body>
            <article>
              <p>This compatibility article has enough repeated explanatory prose to pass extraction thresholds.</p>
              <p>It verifies that the Rust API exposes trafilatura-style helper functions without Python dependencies.</p>
              <p>The result should include content and metadata through the public compatibility aliases.</p>
            </article>
          </body>
        </html>
    "#;

    let options = Options {
        min_extracted_len: 20,
        min_output_size: 5,
        ..Options::default()
    };

    let bare = bare_extraction(html, &options).expect("bare extraction should work");
    assert!(bare.content_text.contains("compatibility article"));

    let with_metadata = extract_with_metadata(html).expect("metadata extraction alias should work");
    assert_eq!(with_metadata.metadata.title.as_deref(), Some("Compatibility Article"));
    assert_eq!(with_metadata.metadata.author.as_deref(), Some("Ada Lovelace"));

    let metadata = extract_metadata(html, &Options::default());
    assert_eq!(metadata.url.as_deref(), Some("https://example.com/article"));
}

#[test]
fn baseline_and_html2txt_match_official_local_helpers() {
    let html = r#"
        <html>
          <body>
            <nav>Home About Contact</nav>
            <fencedframe>Frame noise should be removed</fencedframe>
            <article><p>Baseline extraction keeps the article paragraph for callers.</p></article>
            <script>ignored()</script>
          </body>
        </html>
    "#;

    let baseline = baseline(html);
    assert!(baseline.body_html.starts_with("<body"));
    assert!(baseline.text.contains("Baseline extraction keeps"));
    assert_eq!(baseline.len, baseline.text.chars().count());

    let text = html2txt(html, true);
    assert!(text.contains("Baseline extraction keeps"));
    assert!(!text.contains("Home About Contact"));
    assert!(!text.contains("Frame noise"));
    assert!(!text.contains("ignored"));
}

#[test]
fn load_html_accepts_strings_and_encoded_bytes() {
    let document = load_html("<html><body><p>Hello</p></body></html>");
    assert_eq!(document.select("p").text().to_string(), "Hello");

    let document = load_html_bytes(
        b"<html><head><meta charset=\"ISO-8859-1\"></head><body><p>Caf\xE9</p></body></html>",
    );
    assert_eq!(document.select("p").text().to_string(), "Café");
}

#[test]
fn extraction_results_are_json_serializable() {
    let html = r#"
        <html><body><article><p>Serializable article text with enough words for extraction.</p></article></body></html>
    "#;
    let options = Options {
        min_extracted_len: 10,
        min_output_size: 3,
        ..Options::default()
    };

    let result = bare_extraction(html, &options).expect("extraction should work");
    let json = serde_json::to_string(&result).expect("result should serialize");
    assert!(json.contains("content_text"));
}

#[test]
fn only_with_metadata_requires_title_date_and_url() {
    let html = r#"
        <html>
          <head><title>Missing Date</title><link rel="canonical" href="https://example.com/missing-date"></head>
          <body><article><p>This article lacks the required publication date metadata.</p></article></body>
        </html>
    "#;
    let options = Options {
        only_with_metadata: true,
        min_extracted_len: 10,
        min_output_size: 3,
        ..Options::default()
    };

    let error = bare_extraction(html, &options).expect_err("missing date should reject document");
    assert!(error.to_string().contains("required metadata missing"));
}

#[test]
fn only_with_metadata_accepts_complete_metadata() {
    let html = r#"
        <html>
          <head>
            <title>Complete Metadata</title>
            <meta property="article:published_time" content="2026-07-21T12:00:00Z">
            <link rel="canonical" href="https://example.com/complete">
          </head>
          <body><article><p>This document has title date and URL metadata.</p></article></body>
        </html>
    "#;
    let options = Options {
        only_with_metadata: true,
        min_extracted_len: 10,
        min_output_size: 3,
        ..Options::default()
    };

    let result = bare_extraction(html, &options).expect("complete metadata should pass");
    assert_eq!(result.metadata.title.as_deref(), Some("Complete Metadata"));
    assert_eq!(result.metadata.url.as_deref(), Some("https://example.com/complete"));
    assert!(result.metadata.date.is_some());
}
