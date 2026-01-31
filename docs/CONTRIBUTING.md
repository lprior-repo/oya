# Contributing to llms.txt Standard

Thank you for your interest in contributing to the llms.txt standard and ecosystem!

## Overview

llms.txt is an open standard for AI documentation discovery. This project includes:
- **RFC specification** defining the standard
- **Validator tool** for compliance checking
- **Parser library** for programmatic access
- **Examples** from popular projects
- **Community resources** for adoption

## Ways to Contribute

### 1. Submit Examples

Add llms.txt examples from popular projects:

**What we need:**
- Well-known open-source projects
- Comprehensive documentation sites
- Validated llms.txt files
- Brief description of the project

**Process:**
1. Create `examples/<project>-llms.txt`
2. Follow RFC specification exactly
3. Validate with `llms-txt validate examples/<project>-llms.txt`
4. Add to this document's "Examples" section
5. Submit pull request

**Example projects needed:**
- Go programming language
- Java Spring Framework
- Node.js
- PostgreSQL
- MongoDB
- Vue.js
- Angular
- .NET
- TensorFlow
- PyTorch

### 2. Report Bugs

Found an issue with the validator, parser, or RFC?

1. Check [existing issues](https://github.com/lewisreader/centralized-docs/issues)
2. If not reported, create a new issue
3. Include:
   - Detailed description of the problem
   - Steps to reproduce
   - Expected vs actual behavior
   - Environment (OS, Rust version, etc.)
   - Sample llms.txt file if applicable

### 3. Propose RFC Changes

Want to improve the llms.txt specification?

1. Read the [current RFC](./RFC_LLMS_TXT.md)
2. Create a discussion: [GitHub Discussions](https://github.com/lewisreader/centralized-docs/discussions)
3. Title: `[RFC] Brief description of change`
4. Include:
   - Motivation for the change
   - Proposed changes (with examples)
   - Backward compatibility analysis
   - Impact on existing implementations
5. Gather feedback from community
6. Submit pull request if consensus reached

### 4. Improve Tools

Enhance the validator, parser, or generator:

**Validator (`doc_transformer/src/bin/llms_txt_validator.rs`):**
- Add new validation rules
- Improve error messages
- Add performance optimizations
- Add new output formats (JSON, etc.)

**Parser (`llms-txt-parser/`):**
- Implement in other languages (Python, JavaScript, Go, etc.)
- Add new parsing features
- Improve error handling
- Add serialization/deserialization

**Documentation Tooling:**
- Create plugins for doc generators (mdBook, Docusaurus, Jekyll, Sphinx)
- Create browser extensions
- Build CI/CD validators
- Create linters for common mistakes

### 5. Documentation

Improve project documentation:

- Fix typos or unclear explanations
- Add missing sections
- Improve code examples
- Translate to other languages
- Create tutorials and guides
- Record video demonstrations

## Code of Conduct

### Our Pledge

We are committed to providing a welcoming and inclusive environment for all contributors.

### Our Standards

**Positive behavior:**
- Respectful and inclusive language
- Constructive feedback and collaboration
- Focusing on what's best for the community
- Empathy and understanding

**Unacceptable behavior:**
- Harassment or offensive language
- Personal attacks
- Trolling or disrespectful commentary
- Private or public shaming
- Unwelcome sexual attention or advances
- Publishing others' private information

### Reporting

If you witness or experience unacceptable behavior, report to:
- [GitHub Issues](https://github.com/lewisreader/centralized-docs/issues) with "conduct" label
- Project maintainers will investigate and take action

## Development Workflow

### For Rust Code

1. **Fork the repository**
   ```bash
   # Fork on GitHub, then clone
   git clone https://github.com/YOUR_USERNAME/centralized-docs.git
   cd centralized-docs
   ```

2. **Create a feature branch**
   ```bash
   git checkout -b feature/your-feature-name
   ```

3. **Make changes**
   - Follow existing code style (rustfmt)
   - Write tests for new functionality
   - Ensure no clippy warnings

4. **Test your changes**
   ```bash
   moon run :test      # Run all tests
   moon run :clippy     # Check linting
   moon run :fmt        # Format code
   ```

5. **Commit your changes**
   ```bash
   git add .
   git commit -m "Brief description of changes"
   ```

6. **Push and create PR**
   ```bash
   git push origin feature/your-feature-name
   # Create pull request on GitHub
   ```

### For Documentation

1. **Create a branch** as above
2. **Edit documentation** files in `docs/` or `site/`
3. **Preview** if possible (use Markdown preview tools)
4. **Test links** ensure all links work
5. **Commit and PR** as above

### For Examples

1. **Copy existing example** as template:
   ```bash
   cp examples/rust-llms.txt examples/myproject-llms.txt
   ```

2. **Edit for your project**
   - Update all fields in YAML frontmatter
   - Change project name and description
   - Update URLs to match project
   - Adjust sections and links
   - Ensure format follows RFC exactly

3. **Validate**
   ```bash
   llms-txt validate examples/myproject-llms.txt
   ```

4. **Submit PR**

## Testing

### Running Tests

```bash
# Run all tests
moon run :test

# Run specific test file
cargo test llms_txt_examples_tests

# Run with output
cargo test -- --nocapture
```

### Writing Tests

**Validator tests** (`doc_transformer/src/bin/llms_txt_validator.rs`):
```rust
#[test]
fn test_new_validation_rule() -> anyhow::Result<()> {
    let content = r#"# Test

    ## Getting Started
    - [Link](./path.md): Description
    "#;

    let result = validate_llms_txt(content)?;
    assert!(result.valid);
    Ok(())
}
```

**Parser tests** (`llms-txt-parser/src/lib.rs`):
```rust
#[test]
fn test_new_parsing_feature() -> anyhow::Result<()> {
    let content = r#"---
    llms_version: "1.0"
    ---
    # Test
    "#;

    let llms_txt = parse_content(content)?;
    assert_eq!(llms_txt.frontmatter.version, Some("1.0".to_string()));
    Ok(())
}
```

### Test Coverage

- Validator: Aim for 90%+ coverage
- Parser: Aim for 85%+ coverage
- New features must include tests

## Quality Standards

### Code Quality

- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Zero `unwrap()` or `expect()` calls (use error handling)
- Comprehensive error messages with context
- Public APIs documented with doc comments
- Examples in doc comments for non-trivial functions

### Documentation Quality

- Clear, concise language
- Code examples compile and run
- Links work and go to correct locations
- Examples follow RFC specification
- No jargon without explanation

### Example Quality

All `examples/*.txt` files must:
- Pass `llms-txt validate` with zero errors
- Include all required YAML fields
- Have at least 3 sections (Getting Started, Core Concepts, API Reference)
- Use actual URLs from the project
- Have reasonable descriptions (one sentence per link)

## Pull Request Process

### Before Submitting

- [ ] Tests pass locally
- [ ] Code follows style guidelines
- [ ] Documentation is updated
- [ ] Commit messages are clear
- [ ] PR description explains changes
- [ ] No merge conflicts with main branch

### PR Description Template

```markdown
## Description
Brief description of what this PR does.

## Type
- [ ] Bug fix
- [ ] New feature
- [ ] Documentation
- [ ] Example
- [ ] Refactoring

## Changes
- Bullet point 1
- Bullet point 2

## Testing
- [ ] Unit tests added
- [ ] Integration tests run
- [ ] Manual testing completed

## Checklist
- [ ] Code follows project style
- [ ] Tests pass
- [ ] Documentation updated
- [ ] All examples validated
```

### Review Process

1. **Automated checks** run on every PR
2. **Maintainer review** within 1 week
3. **Community feedback** encouraged
4. **Changes requested** if needed
5. **Approval and merge** when ready

## Recognition

Contributors will be:
- Listed in CONTRIBUTORS.md
- Mentioned in release notes
- Recognized in project README
- Invited to project discussions for major contributors

## License

By contributing, you agree that your contributions will be licensed under:
- **Code:** MIT License
- **Documentation:** CC-BY-4.0 License
- **Examples:** Same license as original project documentation

## Getting Help

- **Questions?** [GitHub Discussions](https://github.com/lewisreader/centralized-docs/discussions)
- **Bugs?** [GitHub Issues](https://github.com/lewisreader/centralized-docs/issues)
- **IRC/Discord?** (Coming soon)

## Examples Already Contributed

See `examples/` directory for validated examples:
- `rust-llms.txt` - Rust Programming Language
- `python-llms.txt` - Python Programming Language
- `kubernetes-llms.txt` - Kubernetes container orchestration
- `docker-llms.txt` - Docker container platform
- `react-llms.txt` - React JavaScript library

## Thank You

Every contribution makes the llms.txt standard better. Thank you for your time and effort!

---

*Last Updated: January 27, 2026*
