# Contributing to R-Droid

Thank you for your interest in contributing to R-Droid! This document provides guidelines and instructions for contributing.

## 🚀 Getting Started

### Prerequisites

1. **Rust 1.75+** - Install from [rustup.rs](https://rustup.rs/)
2. **Git** - For version control
3. **Android SDK** (optional) - Will be auto-downloaded on first run

### Development Setup

```bash
# Clone the repository
git clone https://github.com/r-droid/r-droid.git
cd r-droid

# Install Android targets
rustup target add aarch64-linux-android armv7-linux-androideabi

# Build the project
cargo build

# Run tests
cargo test --workspace

# Run the application
cargo run
```

### IDE Setup

We recommend using VS Code with these extensions:
- rust-analyzer
- Even Better TOML
- CodeLLDB (for debugging)

## 📝 Code Guidelines

### Rust Style

- Follow standard Rust conventions (use `cargo fmt`)
- Run `cargo clippy` before submitting
- Write documentation comments for public APIs
- Use `thiserror` for error types
- Use `tracing` for logging

### Code Structure

```rust
// Good: Clear, documented, error-handled
/// Parses an AndroidManifest.xml file into a structured format.
///
/// # Arguments
/// * `path` - Path to the AndroidManifest.xml file
///
/// # Returns
/// * `Ok(AndroidManifest)` - The parsed manifest
/// * `Err` - If the file cannot be read or parsed
///
/// # Example
/// ```
/// let manifest = ManifestParser::parse("AndroidManifest.xml")?;
/// println!("Package: {}", manifest.package);
/// ```
pub fn parse<P: AsRef<Path>>(path: P) -> Result<AndroidManifest> {
    let content = std::fs::read_to_string(path.as_ref())
        .context("Failed to read manifest file")?;
    
    // ...
}
```

### Error Handling

- Use `anyhow::Result` for application code
- Use `thiserror` for library errors
- Always provide context with `.context()`
- Never use `.unwrap()` in library code

### Async Code

- Use `tokio` as the async runtime
- Prefer `async fn` over manual futures
- Use `tokio::spawn` for concurrent tasks
- Avoid blocking operations in async context

## 🔄 Pull Request Process

### Before Submitting

1. **Create an issue** first for significant changes
2. **Fork** the repository
3. **Create a branch** from `main`:
   ```bash
   git checkout -b feature/your-feature-name
   ```
4. **Make your changes** with clear commits
5. **Run tests**: `cargo test --workspace`
6. **Run lints**: `cargo clippy -- -D warnings`
7. **Format code**: `cargo fmt`

### Commit Messages

Use conventional commits:

```
feat: add manifest visual editor
fix: correct ADB device detection on Windows
docs: update installation instructions
refactor: simplify build configuration
test: add emulator bridge tests
```

### Pull Request Template

```markdown
## Description
Brief description of changes

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Documentation update
- [ ] Refactoring
- [ ] Performance improvement

## Testing
- [ ] Added new tests
- [ ] All existing tests pass
- [ ] Manual testing performed

## Checklist
- [ ] Code follows project style guidelines
- [ ] Self-reviewed the code
- [ ] Added documentation for new features
- [ ] No breaking changes (or documented if breaking)
```

## 🏗️ Architecture Overview

### Crate Responsibilities

| Crate | Purpose |
|-------|---------|
| `r-droid-core` | Orchestration, IPC, configuration |
| `r-droid-ui` | Slint UI components and theming |
| `r-droid-editor` | Code editor with syntax highlighting |
| `r-droid-android-toolchain` | SDK/NDK/JDK management |
| `r-droid-manifest-manager` | AndroidManifest.xml handling |
| `r-droid-emulator-bridge` | Emulator and ADB operations |
| `r-droid-build-engine` | Build system and APK signing |

### Adding a New Feature

1. Identify the appropriate crate
2. Create a new module if needed
3. Define public types in `lib.rs`
4. Write tests alongside implementation
5. Update documentation

### Cross-Crate Communication

Use the core crate's message bus for inter-crate communication:

```rust
use r_droid_core::events::{Event, EventBus};

// Subscribe to events
event_bus.subscribe(|event: BuildComplete| {
    // Handle build completion
});

// Publish events
event_bus.publish(BuildStarted { project: "my-app" });
```

## 🧪 Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_manifest_parsing() {
        let xml = r#"<?xml version="1.0"?>
            <manifest package="com.example.test" />"#;
        
        let manifest = ManifestParser::parse_str(xml).unwrap();
        assert_eq!(manifest.package, "com.example.test");
    }
    
    #[tokio::test]
    async fn test_adb_devices() {
        let client = AdbClient::new().unwrap();
        let devices = client.list_devices().await.unwrap();
        // May be empty if no devices connected
    }
}
```

### Integration Tests

Place in `tests/` directory:

```rust
// tests/build_integration.rs
#[tokio::test]
async fn test_full_build_pipeline() {
    // Set up test project
    // Run build
    // Verify APK output
}
```

### Running Tests

```bash
# All tests
cargo test --workspace

# Specific crate
cargo test -p r-droid-manifest-manager

# With output
cargo test -- --nocapture

# Ignored tests (require setup)
cargo test -- --ignored
```

## 📖 Documentation

### Adding Documentation

- Use `///` for public API documentation
- Use `//!` for module-level documentation
- Include examples in doc comments
- Update README.md for user-facing changes

### Building Docs

```bash
cargo doc --workspace --no-deps --open
```

## 🐛 Bug Reports

### Good Bug Report

```markdown
## Description
Clear description of the bug

## Steps to Reproduce
1. Open R-Droid
2. Click on ...
3. Observe error

## Expected Behavior
What should happen

## Actual Behavior
What actually happens

## Environment
- R-Droid version: 0.1.0
- OS: Windows 11
- Android SDK: 34.0.0
```

## 💡 Feature Requests

### Good Feature Request

```markdown
## Problem Statement
What problem does this solve?

## Proposed Solution
How would this feature work?

## Alternatives Considered
Other approaches you've thought about

## Additional Context
Mockups, examples, or references
```

## 📜 License

By contributing to R-Droid, you agree that your contributions will be licensed under the MIT License.

## 🙏 Thank You!

Every contribution helps make R-Droid better. Whether it's:
- Fixing a typo
- Reporting a bug
- Suggesting a feature
- Writing code
- Improving documentation

**You're making a difference!**
