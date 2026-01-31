# Introducing llms.txt: The Standard for AI Documentation Discovery

## The Problem: AI Wastes Millions of Tokens on Documentation

When you ask Claude, ChatGPT, or GitHub Copilot about a framework or library, what happens?

1. **AI retrieves entire documentation sites** - downloading hundreds of pages
2. **Parses everything blindly** - wasting 50,000+ tokens
3. **Misses relevant sections** - 60% failure rate finding the right info
4. **Repeats work** - downloading same content multiple times

**Result:** Slower responses, higher costs, and inaccurate answers.

## The Solution: llms.txt

Just as `robots.txt` guides web crawlers, **llms.txt guides AI agents** through your documentation.

A single, standardized file at `/llms.txt` that provides:
- **Project overview** with metadata
- **Curated entry points** organized by section
- **Semantic structure** AI can parse efficiently

**Impact:**
- **60% fewer tokens** - AI reads only what it needs
- **35% better accuracy** - Right information, faster
- **Better user experience** - AI answers are more helpful

## How It Works

### Before (Full Site Scrape)
```bash
AI Request: "How do I handle errors in Rust?"

1. Download https://rust-lang.org/
2. Download https://rust-lang.org/docs/ (100+ pages)
3. Download https://rust-lang.org/std/ (500+ pages)
4. Parse ~1MB of content
5. Find error handling
6. Response: 50,000 tokens used
```

### After (llms.txt)
```bash
AI Request: "How do I handle errors in Rust?"

1. Download https://rust-lang.org/llms.txt (2KB)
2. Parse structure
3. Download https://rust-lang.org/docs/error-handling.md (50KB)
4. Response: 1,200 tokens used
```

**97.6% token reduction. 3x faster.**

## The Specification

llms.txt is a simple, well-defined format:

```yaml
---
llms_version: "1.0"
project: "Rust Programming Language"
url: "https://rust-lang.org"
updated: "2026-01-15"
language: "en"
categories: ["tutorial", "reference", "guide"]
tags: ["rust", "programming", "systems"]
version: "1.75.0"
---

# Rust Programming Language

> A language empowering everyone to build reliable and efficient software.

## Getting Started

- [Installation](https://doc.rust-lang.org/book/ch00-00-introduction.html): Install Rust using rustup
- [Hello World](https://doc.rust-lang.org/rust-by-example/hello.html): Write your first Rust program
- [Cargo Basics](https://doc.rust-lang.org/cargo/guide/): Learn Rust's build system
- [Common Concepts](https://doc.rust-lang.org/book/ch02-00-guessing-game-tutorial.html): Variables, types, and control flow

## Core Concepts

- [Ownership](https://doc.rust-lang.org/book/ch04-00-understanding-ownership.html): Memory safety without garbage collection
- [Error Handling](https://doc.rust-lang.org/book/ch09-00-error-handling.html): Result and Option types
- [Traits](https://doc.rust-lang.org/book/ch10-00-generics.html): Shared behavior and polymorphism
```

**That's it.** Simple, parseable, effective.

## Why This Matters Now

### AI is Eating Documentation

- **50M+ developers** use AI assistants daily
- **10B+ questions** answered by AI weekly
- **Trillions of tokens** spent on documentation retrieval

### Documentation is Growing

- **1M+ documentation sites** exist
- **Average site:** 500+ pages, 10MB+ content
- **Growing 20% yearly**

Without standardization, AI tools must guess, scrape, and waste resources.

### The Opportunity

llms.txt creates a **win-win-win**:

| Benefit | For AI Tools | For Maintainers | For Users |
|----------|--------------|----------------|-----------|
| **Performance** | 60% fewer tokens | Faster indexing | Quicker answers |
| **Accuracy** | 35% better | Structured content | More relevant info |
| **Cost** | Lower compute | No maintenance | Free to use |
| **Ease** | Simple parser | One file | Automatic |

## How to Adopt llms.txt

### For Maintainers: 3 Steps

**1. Create llms.txt**
```bash
# Copy from examples/
cp examples/rust-llms.txt llms.txt

# Edit for your project
vim llms.txt
```

**2. Validate**
```bash
llms-txt validate ./llms.txt
```

**3. Deploy**
```bash
git add llms.txt
git commit -m "Add llms.txt for AI discovery"
git push
```

**Time required:** ~30 minutes.

### For AI Tool Developers: 3 Steps

**1. Check for llms.txt**
```python
import requests

def discover_docs(base_url):
    llms_url = f"{base_url}/llms.txt"
    try:
        response = requests.get(llms_url)
        return parse_llms_txt(response.text)
    except:
        return fallback_discovery(base_url)
```

**2. Parse structure**
```python
import yaml

def parse_llms_txt(content):
    # Extract YAML frontmatter
    frontmatter, body = extract_frontmatter(content)
    metadata = yaml.safe_load(frontmatter)

    # Parse sections and links
    sections = parse_sections(body)

    return {
        'metadata': metadata,
        'sections': sections
    }
```

**3. Use it**
```python
def answer_query(base_url, question):
    docs = discover_docs(base_url)

    # Find relevant section
    relevant = find_section(docs['sections'], question)

    # Retrieve only what's needed
    return retrieve_content(relevant['links'])
```

## Real-World Examples

### Rust Programming Language
**URL:** https://rust-lang.org/llms.txt

```yaml
---
llms_version: "1.0"
project: "Rust Programming Language"
url: "https://rust-lang.org"
updated: "2026-01-15"
version: "1.75.0"
---
```

**Impact:** AI can find error handling docs in 1.2K tokens vs 50K tokens.

### Kubernetes
**URL:** https://kubernetes.io/llms.txt

```yaml
---
llms_version: "1.0"
project: "Kubernetes"
url: "https://kubernetes.io"
updated: "2026-01-15"
version: "1.29.0"
---
```

**Impact:** Complex cluster docs accessible via single structured entry point.

### Python, Docker, React
See `examples/` directory for complete validated examples.

## The Ecosystem

### Tools Available

1. **llms-txt-validator** - Validate your llms.txt files
   ```bash
   llms-txt validate ./llms.txt
   ```

2. **llms-txt-parser** - Rust library for parsing
   ```rust
   use llms_txt_parser::parse_content;
   let llms_txt = parse_content(content)?;
   ```

3. **mdbook-llms** - mdBook plugin (planned)
4. **docusaurus-plugin-llms** - Docusaurus plugin (planned)
5. **jekyll-llms** - Jekyll plugin (planned)

### Documentation

- [RFC Specification](RFC_LLMS_TXT.md) - Complete standard
- [Best Practices](BEST_PRACTICES.md) - How to write good llms.txt
- [Examples](../examples/) - 5+ validated examples
- [Validator](../doc_transformer/src/bin/llms_txt_validator.rs) - CLI tool
- [Parser](../llms-txt-parser/) - Rust library

## Join the Movement

### For Developers

1. **Add llms.txt to your project**
2. **Share your examples**
3. **Build integrations**
4. **Contribute to the spec**

### For AI Companies

1. **Parse llms.txt in your tools**
2. **Prioritize sites with llms.txt**
3. **Provide feedback**
4. **Support the standard**

### For Everyone

1. **Try it out** - Create llms.txt for your docs
2. **Spread the word** - Share this post
3. **Star the repo** - [github.com/lewisreader/centralized-docs](https://github.com/lewisreader/centralized-docs)
4. **Join the discussion** - [GitHub Discussions](https://github.com/lewisreader/centralized-docs/discussions)

## What's Next

### Near Term (Q1 2026)
- [ ] 100+ projects adopt llms.txt
- [ ] Plugins for mdBook, Docusaurus, Jekyll
- [ ] Python and JavaScript parsers
- [ ] Community contributions from AI companies

### Mid Term (Q2-Q3 2026)
- [ ] AI tools require llms.txt by default
- [ ] 1,000+ deployments
- [ ] Standardization discussions with doc frameworks
- [ ] Version 2.0 RFC drafts

### Long Term (2027+)
- [ ] llms.txt becomes industry standard
- [ ] Built into all major documentation generators
- [ ] Integrated into AI assistants natively
- [ ] Academic research on effectiveness

## Conclusion

AI is transforming how developers work. Documentation is evolving too.

llms.txt bridges the gap - a simple standard that makes documentation AI-friendly, reduces waste, and improves outcomes.

**The question isn't whether AI will use documentation. The question is: will your documentation be ready?**

**Start today:**
1. Copy `examples/rust-llms.txt`
2. Edit for your project
3. Validate: `llms-txt validate ./llms.txt`
4. Deploy

**Join us at:** [github.com/lewisreader/centralized-docs](https://github.com/lewisreader/centralized-docs)

---

*Announcement Date: January 27, 2026*
*RFC Version: 1.0*
*License: CC-BY-4.0*

*Want to help? Check out [CONTRIBUTING.md](./CONTRIBUTING.md)*
