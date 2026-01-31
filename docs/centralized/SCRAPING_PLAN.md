# Web Scraping Improvement Plan

## Current Issues

### 1. Rate Limit Handling
- **Problem**: Scraper saves "Rate limit exceeded" pages as content (526/1514 = 34.7%)
- **Current Fix**: Detect and skip rate limit pages (partial solution)
- **Better Solution**: Use spider-rs built-in retry + exponential backoff

### 2. Context Prefixes
- **Status**: ✅ FIXED - Added `context_prefix` field to Chunk struct
- **Implementation**: Uses `context_buffer` already in chunking logic
- **Testing**: Need to verify in generated chunks

## Better Approaches

### Option A: Use spider-rs Native Features

Spider-rs 2.39 has built-in features we should leverage:

```rust
// Already available in spider::configuration:
website.configuration.delay = config.delay_ms;  // ✅ We use this
website.configuration.respect_robots_txt = true;  // ✅ We use this

// Should add:
website.configuration.request_timeout = Some(Duration::from_secs(30));
website.configuration.retry_strategy = Some(RetryStrategy::ExponentialBackoff {
    max_retries: 3,
    base_delay: Duration::from_millis(1000),
});
```

### Option B: Git Clone (Recommended for Documentation)

**Why this is better:**
- No rate limiting ever
- Get raw markdown (no HTML parsing issues)
- Faster (no HTTP delays needed)
- Works offline after clone
- Clean history (git tags/branches)

**Implementation:**

```bash
doc_transformer ingest-git https://github.com/nushell/nushell.github.io \
  --output ./indexed \
  --branch main \
  --depth 1
```

**Pros:**
- ✅ Zero network errors
- ✅ Perfect markdown (source files)
- ✅ Instant (git clone is optimized)
- ✅ Can update with `git pull`
- ✅ Supports any language (not just markdown)

**Cons:**
- ❌ Only works for Git-hosted docs (not all sites)
- ❌ Requires git CLI installed

## Proposed Changes

### Phase 1: Fix Scraping (Quick Win)

1. **Add spider-rs retry configuration**
   ```rust
   // In scrape.rs
   website.configuration.request_timeout = Some(Duration::from_secs(30));
   // Note: spider-rs may not expose retry_strategy directly
   // Verify in docs or source
   ```

2. **Improve rate limit detection**
   - Currently: Skip pages on first detection
   - Better: Track consecutive rate limits, abort after N failures

### Phase 2: Add Git Ingest (Best for Docs)

1. **New subcommand: `ingest-git`**
   ```rust
   Commands::IngestGit {
       /// Git repository URL to clone
       url: String,

       /// Output directory for indexed content
       #[arg(short, long)]
       output: PathBuf,

       /// Git branch to checkout (default: main)
       #[arg(long)]
       branch: Option<String>,

       /// Clone depth (1 = shallow, faster)
       #[arg(long)]
       depth: Option<usize>,
   }
   ```

2. **Implementation steps:**
   ```rust
   // 1. Clone repo
   Command::new("git")
       .args(["clone", "--depth", "1", url, temp_dir])
       .status()?;

   // 2. Find all .md files
   let markdown_files = WalkDir::new(temp_dir)
       .filter(|e| e.path().extension() == Some("md"))
       .collect();

   // 3. Use existing index pipeline
   run_index(&markdown_files, &output, &config)
   ```

3. **Update README:**
   ```bash
   # For Git-hosted docs (recommended!):
   doc_transformer ingest-git https://github.com/nushell/nushell.github.io \
     --output ./nushell_docs

   # For web scraping (when no Git access):
   doc_transformer scrape https://docs.example.com \
     --output ./scraped
   ```

## Decision Matrix

| Scenario | Best Approach | Why |
|-----------|---------------|------|
| GitHub/GitLab/Bitbucket docs | **Git Clone** | Zero errors, fast, reliable |
| Private Git repos | **Git Clone** | Works with auth, no scraping needed |
| Static sites without Git | **Web Scraping** | Only option, use spider-rs retries |
| Dynamic content (SPA) | **Web Scraping** | Git may not have rendered content |
| Mixed (git + generated pages) | **Both** | Git for docs, scrape for API references |

## Recommendation

**Implement Option B (Git Ingest) as primary**, keep Option A (Scraping) as fallback:

1. Docs with Git access → Use `ingest-git` (fast, error-free)
2. Docs without Git → Use `scrape` with better retry logic

This mirrors industry best practices:
- **ReadTheDocs**: Offers Git access for all projects
- **GitHub Pages**: All in Git repos
- **Docusaurus**: Often deployed from Git

Most documentation worth indexing is already in Git repos!
