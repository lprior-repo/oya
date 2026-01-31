# Best Practices for llms.txt

This guide helps you create effective llms.txt files that AI agents can parse and use efficiently.

## Writing Effective Link Descriptions

### Keep it Short
Each link description should be one sentence (max ~100 characters).

**Good:**
```markdown
- [Ownership](./ownership.md): Understand memory safety without garbage collection
```

**Bad:**
```markdown
- [Ownership](./ownership.md): Ownership is a core concept in Rust that ensures memory safety by tracking which part of code owns each piece of data, eliminating the need for garbage collection.
```

### Focus on Value
Explain what the reader will gain, not just what the content is.

**Good:**
```markdown
- [Installation](./install.md): Get started in 5 minutes
```

**Bad:**
```markdown
- [Installation](./install.md): This page contains installation instructions
```

### Use Active Voice
Active descriptions are more engaging and clearer.

**Good:**
```markdown
- [Deployment](./deploy.md): Deploy to production in 3 steps
```

**Bad:**
```markdown
- [Deployment](./deploy.md): Deployment is explained in this section
```

## Structuring Sections

### Logical Grouping
Group related topics together in intuitive sections.

**Recommended order:**
1. Getting Started - First steps for beginners
2. Core Concepts - Essential understanding
3. API Reference - Technical details
4. Operations - Production concerns
5. Advanced Topics - Specialized topics

### Progressive Complexity
Organize sections and links from simple to complex.

**Example - Core Concepts:**
```markdown
## Core Concepts

- [Variables](./variables.md): Storing and using data
- [Functions](./functions.md): Reusable code blocks
- [Modules](./modules.md): Organizing code
- [Traits](./traits.md): Shared behavior
- [Macros](./macros.md): Code generation
```

### Section Balance
Aim for 3-5 links per section, 4-6 sections total.

**Too sparse:**
```markdown
## Getting Started

- [Install](./install.md)
```

**Too dense:**
```markdown
## Getting Started

- [Install](./install.md)
- [Configure](./config.md)
- [Initialize](./init.md)
- [Build](./build.md)
- [Test](./test.md)
- [Deploy](./deploy.md)
- [Monitor](./monitor.md)
- [Troubleshoot](./troubleshoot.md)
```

**Just right:**
```markdown
## Getting Started

- [Installation](./install.md): Get started in 5 minutes
- [Quick Start](./quickstart.md): Build your first app
- [Configuration](./config.md): Set up your environment
```

## Choosing Appropriate Metadata

### Required Fields
Always include these in YAML frontmatter:
- `llms_version`: RFC version (e.g., "1.0")
- `project`: Project name (e.g., "React")
- `url`: Base URL (e.g., "https://react.dev")
- `updated`: Last update date (ISO 8601: "2026-01-15")

### Optional but Recommended
These help AI agents understand your docs better:
- `language`: Primary language (ISO 639-1: "en", "es", "zh")
- `categories`: Document categories (e.g., ["tutorial", "reference"])
- `tags`: Relevant keywords (e.g., ["rust", "systems"])
- `version`: Documentation version (e.g., "18.2.0")

### Example: Well-structured Frontmatter
```yaml
---
llms_version: "1.0"
project: "React"
url: "https://react.dev"
updated: "2026-01-15"
language: "en"
categories: ["tutorial", "reference", "guide"]
tags: ["react", "javascript", "frontend", "ui"]
version: "18.2.0"
---
```

## Common Pitfalls and How to Avoid Them

### 1. Missing Required Sections
**Problem:** AI can't find essential information.

**Solution:** Always include "Getting Started" and "Core Concepts".

```markdown
## Getting Started
- [Installation](./install.md): Get up and running

## Core Concepts
- [Components](./components.md): Building blocks
```

### 2. Poor URL Resolution
**Problem:** Links break or are ambiguous.

**Solution:** Use absolute URLs for external resources, relative for internal.

```markdown
## Getting Started

# Good: External resource
- [Official Site](https://react.dev): Visit React website

# Good: Internal resource
- [Tutorial](./tutorial/getting-started.md): First steps

# Bad: Ambiguous
- [Docs](../docs.html): Hard to resolve
```

### 3. Inconsistent Formatting
**Problem:** AI parsers struggle with inconsistent styles.

**Solution:** Follow the RFC exactly - no variations.

```markdown
# Correct format
- [Title](./path.md): Description

# Common mistakes
[Title](./path.md)                    # Missing description
- [Title](./path.md)                   # Missing colon
- Title (./path.md)                    # Wrong bracket placement
- [Title](./path.md) - Description      # Wrong separator
```

### 4. Oversized Files
**Problem:** File exceeds 50KB limit.

**Solution:** Curate content, link to details.

```markdown
# Don't include full content
## Core Concepts

Ownership is Rust's most unique feature. It enables memory safety without
garbage collection by enforcing compile-time rules about which part of the
program owns each piece of data. This means no data races, no dangling
pointers, and no memory leaks... [3000 words]

# Instead, provide overview and links
## Core Concepts

- [Ownership](./ownership.md): Understand memory safety without GC
- [Borrowing](./borrowing.md): References and lifetimes
- [Traits](./traits.md): Shared behavior and polymorphism
```

### 5. Missing Project Context
**Problem:** AI doesn't know what the project is.

**Solution:** Include clear project description.

```markdown
# Rust Programming Language

> A language empowering everyone to build reliable and efficient software.

Rust is a systems programming language focused on safety, speed, and concurrency.
```

## Integration with Existing Doc Generators

### mdBook
```bash
# Install mdbook-llms plugin
cargo install mdbook-llms

# Generate llms.txt from SUMMARY.md
mdbook build --llms-output llms.txt
```

### Docusaurus
```javascript
// docusaurus.config.js
module.exports = {
  plugins: [
    [
      'docusaurus-plugin-llms',
      {
        outputFile: 'llms.txt',
        sections: ['docs', 'api']
      }
    ]
  ]
}
```

### Jekyll
```liquid
<!-- llms.txt -->
---
llms_version: "1.0"
project: "{{ site.title }}"
url: "{{ site.url }}"
updated: "{{ site.time | date: "%Y-%m-%d" }}"
---

# {{ site.title }}

> {{ site.description }}

## Getting Started
{% for doc in site.docs limit:5 %}
- [{{ doc.title }}]({{ doc.url }}): {{ doc.excerpt }}
{% endfor %}
```

### Sphinx
```python
# conf.py
def setup(app):
    app.add_config_value('llms_output', 'llms.txt', 'html')
```

## Validation

Always validate your llms.txt before deployment:

```bash
# Using llms-txt-validator
llms-txt validate ./llms.txt

# With strict warnings
llms-txt validate --strict ./llms.txt

# CI/CD integration
- run: llms-txt validate ./llms.txt
```

## Tips for Adoption

### For Project Maintainers
1. Start with core sections (Getting Started, Core Concepts)
2. Add examples incrementally
3. Gather feedback from AI tool users
4. Keep file under 10KB for optimal performance

### For Users
1. Check for llms.txt before asking questions
2. Use it to navigate large documentation sites
3. Provide feedback to maintainers about usefulness

### For AI Tool Developers
1. Parse YAML frontmatter first
2. Fall back gracefully if optional fields missing
3. Cache parsed llms.txt files
4. Respect section hierarchy and link order

## Example: Complete llms.txt

See `examples/rust-llms.txt`, `examples/python-llms.txt`, and other example files in this repository for complete, validated examples.

## Resources

- [RFC Specification](./RFC_LLMS_TXT.md) - Complete specification
- [Validator Tool](../doc_transformer/src/bin/llms_txt_validator.rs) - CLI validator
- [Parser Library](../llms-txt-parser) - Rust parser implementation
- [Examples Directory](../examples/) - Validated examples from popular projects

## Getting Help

- [GitHub Issues](https://github.com/lewisreader/centralized-docs/issues) - Report bugs
- [Discussions](https://github.com/lewisreader/centralized-docs/discussions) - Ask questions
- [Contributing Guide](./CONTRIBUTING.md) - Contribute to the project
