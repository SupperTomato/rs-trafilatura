//! Performance benchmarks for rs-trafilatura.
//!
//! Run with: `cargo bench`
//!
//! Benchmarks include:
//! - Small synthetic HTML (~1KB) for microbenchmarks
//! - Real-world HTML files from benchmark dataset for realistic performance

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rs_trafilatura::{baseline, extract, extract_with_options, html2txt, Options};
use std::fs;

const SAMPLE_HTML: &str = r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Sample Article</title>
    <meta name="author" content="John Doe">
    <meta name="description" content="A sample article for benchmarking.">
</head>
<body>
    <nav>
        <a href="/">Home</a>
        <a href="/about">About</a>
    </nav>
    <article>
        <h1>Sample Article Title</h1>
        <p class="byline">By John Doe</p>
        <p>This is the first paragraph of the article. It contains some meaningful
        content that should be extracted by the trafilatura algorithm.</p>
        <p>Here is a second paragraph with more content. The extraction should
        preserve the text while removing navigation and other boilerplate.</p>
        <p>A third paragraph ensures we have enough content for meaningful
        benchmarking of the extraction performance.</p>
    </article>
    <aside>
        <h3>Related Articles</h3>
        <ul>
            <li>Related article 1</li>
            <li>Related article 2</li>
        </ul>
    </aside>
    <footer>
        <p>Copyright 2024</p>
    </footer>
</body>
</html>
"#;

const JSON_LD_HTML: &str = r#"
<!DOCTYPE html>
<html>
<head>
    <title>Structured Data Article</title>
    <script type="application/ld+json">
    {
        "@context": "https://schema.org",
        "@type": "FAQPage",
        "mainEntity": {
            "@type": "Question",
            "name": "How does extraction work?",
            "acceptedAnswer": {
                "@type": "Answer",
                "text": "Rust extraction parses the DOM, scores content candidates, and falls back to structured JSON-LD text when visible content is not sufficient."
            }
        },
        "recipeInstructions": [
            {"text": "Parse the document once."},
            {"itemListElement": [{"text": "Collect structured body text."}]}
        ]
    }
    </script>
    <script type="application/ld+json">
    {
        "@context": "https://schema.org",
        "@type": "Product",
        "description": "This product teaser is available only as structured metadata and should be kept as fallback text."
    }
    </script>
</head>
<body><nav>Home Products Cart</nav></body>
</html>
"#;

const METADATA_ONLY_JSON_LD_HTML: &str = r#"
<!DOCTYPE html>
<html>
<head>
    <title>Metadata Only</title>
    <script type="application/ld+json">
    {"@context":"https://schema.org","@type":"Organization","name":"Example","url":"https://example.com"}
    </script>
    <script type="application/ld+json">
    {"@context":"https://schema.org","@type":"BreadcrumbList","itemListElement":[{"name":"Home"},{"name":"Section"}]}
    </script>
    <script type="application/ld+json">
    {"@context":"https://schema.org","@type":"WebSite","name":"Example","potentialAction":{"@type":"SearchAction"}}
    </script>
</head>
<body>
    <article>
        <p>Fallback should eventually use visible article text when metadata-only JSON-LD contains no content body.</p>
        <p>This benchmark measures the cost of scanning irrelevant structured data scripts.</p>
        <p>The optimized path should skip JSON parsing when no content-bearing hook is present.</p>
    </article>
</body>
</html>
"#;

fn bench_extract_default(c: &mut Criterion) {
    c.bench_function("extract_default", |b| {
        b.iter(|| extract(black_box(SAMPLE_HTML)));
    });
}

fn bench_extract_with_options(c: &mut Criterion) {
    let options = Options {
        include_tables: true,
        favor_precision: true,
        ..Options::default()
    };

    c.bench_function("extract_with_options", |b| {
        b.iter(|| extract_with_options(black_box(SAMPLE_HTML), black_box(&options)));
    });
}

fn bench_official_api_helpers(c: &mut Criterion) {
    let mut group = c.benchmark_group("official_api");
    group.throughput(Throughput::Bytes(SAMPLE_HTML.len() as u64));

    group.bench_function("baseline_sample", |b| {
        b.iter(|| baseline(black_box(SAMPLE_HTML)));
    });

    group.bench_function("html2txt_clean_sample", |b| {
        b.iter(|| html2txt(black_box(SAMPLE_HTML), black_box(true)));
    });

    group.throughput(Throughput::Bytes(JSON_LD_HTML.len() as u64));
    group.bench_function("baseline_json_ld", |b| {
        b.iter(|| baseline(black_box(JSON_LD_HTML)));
    });

    group.throughput(Throughput::Bytes(METADATA_ONLY_JSON_LD_HTML.len() as u64));
    group.bench_function("baseline_metadata_only_json_ld", |b| {
        b.iter(|| baseline(black_box(METADATA_ONLY_JSON_LD_HTML)));
    });

    group.finish();
}

/// Benchmark with real-world HTML files of varying sizes
fn bench_real_world_html(c: &mut Criterion) {
    let html_dir = "../data/html_files";

    // Try to load sample files of different sizes
    let sample_files = ["0001.html", "0010.html", "0100.html"];

    let mut group = c.benchmark_group("real_world");

    for filename in &sample_files {
        let path = format!("{html_dir}/{filename}");
        if let Ok(html) = fs::read_to_string(&path) {
            let size_kb = html.len() / 1024;
            group.throughput(Throughput::Bytes(html.len() as u64));
            group.bench_with_input(
                BenchmarkId::new("extract", format!("{filename} ({size_kb}KB)")),
                &html,
                |b, html| {
                    b.iter(|| extract(black_box(html)));
                },
            );
        }
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_extract_default,
    bench_extract_with_options,
    bench_official_api_helpers,
    bench_real_world_html
);
criterion_main!(benches);
