# Installation Guide

## Prerequisites

Before installing Intent, ensure you have:

- **Gleam** (1.0 or later) - [Install Gleam](https://gleam.run/getting-started/installing-gleam/)
- **Erlang/OTP 27** - Required by Gleam
- **Git** - For cloning the repository

### Checking Prerequisites

```bash
# Check Gleam version
gleam --version

# Check Erlang version
erl -eval 'erlang:display(erlang:system_info(otp_release)), halt().' -noshell
```

## Installation Methods

### Method 1: From Source (Recommended for Development)

```bash
# Clone the repository
git clone https://github.com/yourusername/intent-cli
cd intent-cli

# Build the project
gleam build

# (Optional) Add to PATH for convenient access
export PATH="$PATH:$(pwd)/build/dev/erlang"
```

### Method 2: Using Package Manager (When Available)

```bash
# Gleam package manager (when published to hex.pm)
gleam add intent
```

## Verifying Installation

To verify Intent is properly installed:

```bash
# Should display the Intent CLI version and help information
gleam run -- --help
```

You should see output like:

```
Intent CLI - Contract-Driven API Testing

Usage: intent [COMMAND] [OPTIONS]

Commands:
  check - Validate an API specification against a running server

Options:
  --help, -h      Show this help message
  --version, -v   Show version information
```

## Quick Test

To ensure everything is working correctly:

```bash
# Create a simple test spec
cat > test-api.cue << 'EOF'
package api

spec: {
    name: "Test API"
    description: "Simple test"
    audience: "Everyone"
    version: "1.0.0"

    config: {
        base_url: "https://httpbin.org"
        timeout_ms: 5000
        headers: {}
    }

    features: [{
        name: "Basic"
        description: "Basic test"
        behaviors: [{
            name: "get-root"
            intent: "Get the root endpoint"
            request: {
                method: "GET"
                path: "/get"
                headers: {}
                query: {}
                body: null
            }
            response: {
                status: 200
                example: { url: "https://httpbin.org/get" }
                checks: {}
                headers: {}
            }
            captures: {}
        }]
    }]

    rules: []
    anti_patterns: []
    success_criteria: []
    ai_hints: {
        implementation: { suggested_stack: [] }
        entities: {}
        security: {
            password_hashing: ""
            jwt_algorithm: ""
            jwt_expiry: ""
            rate_limiting: ""
        }
        pitfalls: []
    }
}
EOF

# Run Intent against the test API
gleam run -- check test-api.cue --target https://httpbin.org
```

## Troubleshooting

### "gleam: command not found"

Gleam is not installed or not in your PATH.

**Solution:**
- Install Gleam from https://gleam.run/getting-started/installing-gleam/
- Ensure the Gleam installation directory is in your PATH

### "Erlang OTP version too old"

Intent requires Erlang/OTP 27 or later.

**Solution:**
```bash
# Check your Erlang version
erl -eval 'erlang:display(erlang:system_info(otp_release)), halt().' -noshell

# If version is too old, install Erlang/OTP 27
# On macOS with Homebrew:
brew install erlang@27

# On Ubuntu/Debian:
sudo apt-get install erlang-27

# On other systems, see: https://www.erlang.org/downloads
```

### "Failed to compile"

There might be a compatibility issue or missing dependency.

**Solution:**
```bash
# Clean build artifacts
gleam clean

# Try building again
gleam build

# Check for error messages and report to the project
```

### "HTTP request failed" when running tests

The API server might not be running or the URL is incorrect.

**Solution:**
- Ensure your API server is running
- Check that the `--target` URL is correct
- Verify network connectivity

## Development Setup

For contributors who want to develop Intent itself:

```bash
# Clone the repository
git clone https://github.com/yourusername/intent-cli
cd intent-cli

# Install dependencies
gleam build

# Run tests
gleam test

# Set up pre-commit hooks (optional)
git config core.hooksPath .hooks
```

## Next Steps

After installation, check out:
- [User Guide](USER_GUIDE.md) - Learn how to use Intent
- [CUE Specification Format](SPEC_FORMAT.md) - Understand the specification syntax
- [Examples](../examples/) - Explore example specifications

## Support

If you encounter any issues during installation:

1. Check this troubleshooting section
2. Review the [FAQ](FAQ.md)
3. Open an [Issue](https://github.com/yourusername/intent-cli/issues) on GitHub
4. Ask in [Discussions](https://github.com/yourusername/intent-cli/discussions)
