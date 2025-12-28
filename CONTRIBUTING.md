# Contributing to rpview-gpui

Thank you for your interest in contributing to rpview-gpui! This document provides guidelines and information for contributors.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Project Structure](#project-structure)
- [Development Workflow](#development-workflow)
- [Coding Standards](#coding-standards)
- [Testing](#testing)
- [Submitting Changes](#submitting-changes)

## Code of Conduct

This project adheres to a code of conduct that all contributors are expected to follow:

- Be respectful and inclusive
- Welcome newcomers and help them get started
- Focus on constructive feedback
- Assume good intentions
- Respect differing viewpoints and experiences

## Getting Started

1. **Fork the repository** on GitHub
2. **Clone your fork** locally:
   ```bash
   git clone https://github.com/YOUR_USERNAME/rpview-gpui.git
   cd rpview-gpui/rpview-gpui
   ```
3. **Add the upstream repository**:
   ```bash
   git remote add upstream https://github.com/ORIGINAL_OWNER/rpview-gpui.git
   ```

## Development Setup

### Prerequisites

- Rust (latest stable version)
- Cargo (comes with Rust)
- Platform-specific dependencies for GPUI:
  - **macOS**: Xcode Command Line Tools
  - **Linux**: Development packages for X11, etc.
  - **Windows**: Visual Studio Build Tools

### Building

```bash
cargo build
```

### Running

```bash
cargo run -- [OPTIONS] [PATHS...]
```

Example:
```bash
cargo run -- test_images/
cargo run -- image1.png image2.jpg
```

## Project Structure

```
rpview-gpui/
├── src/
│   ├── main.rs           # Application entry point
│   ├── error.rs          # Error types and handling
│   ├── cli.rs            # Command-line argument parsing
│   ├── state/            # Application and image state management
│   │   ├── mod.rs
│   │   ├── app_state.rs
│   │   └── image_state.rs
│   ├── components/       # UI components
│   │   ├── mod.rs
│   │   ├── image_viewer.rs
│   │   └── ...
│   └── utils/            # Utility modules
│       ├── mod.rs
│       ├── style.rs      # Styling utilities
│       └── ...
├── Cargo.toml
├── TODO.md               # Development roadmap
├── DESIGN.md             # Application design documentation
└── CLI.md                # CLI interface documentation
```

## Development Workflow

### 1. Create a Feature Branch

```bash
git checkout -b feature/your-feature-name
```

### 2. Make Your Changes

- Follow the coding standards (see below)
- Write clear, descriptive commit messages
- Add tests for new functionality
- Update documentation as needed

### 3. Test Your Changes

```bash
cargo test
cargo clippy
cargo fmt --check
```

### 4. Commit Your Changes

```bash
git add .
git commit -m "Add feature: description of your changes"
```

Use clear commit messages:
- Start with a verb (Add, Fix, Update, Remove, etc.)
- Be concise but descriptive
- Reference issue numbers when applicable

### 5. Keep Your Branch Updated

```bash
git fetch upstream
git rebase upstream/main
```

### 6. Push to Your Fork

```bash
git push origin feature/your-feature-name
```

## Coding Standards

### Rust Style

- Follow the [Rust Style Guide](https://doc.rust-lang.org/1.0.0/style/)
- Use `cargo fmt` to format code automatically
- Run `cargo clippy` and address all warnings
- Use meaningful variable and function names
- Add documentation comments (`///`) for public APIs

### Code Organization

- Keep functions focused and single-purpose
- Use modules to organize related functionality
- Separate concerns (UI, state, business logic)
- Avoid deep nesting; refactor complex logic into functions

### Error Handling

- Use the `AppError` and `AppResult` types from `error.rs`
- Provide descriptive error messages
- Handle errors gracefully; don't panic in normal operation
- Use `?` operator for error propagation

### Comments and Documentation

- Document all public APIs with `///` comments
- Add inline comments for complex logic
- Keep comments up-to-date with code changes
- Write clear README sections for new features

## Testing

### Unit Tests

Place unit tests in the same file as the code they test:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_something() {
        // Test code
    }
}
```

### Integration Tests

Place integration tests in the `tests/` directory (to be created).

### Running Tests

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name
```

## Submitting Changes

### Pull Request Process

1. **Ensure all tests pass** and code is formatted
2. **Update documentation** if needed (README, DESIGN.md, TODO.md)
3. **Create a pull request** on GitHub
4. **Write a clear PR description**:
   - What changes does this PR introduce?
   - Why are these changes needed?
   - How have you tested these changes?
   - Reference any related issues

### PR Review Process

- Maintainers will review your PR
- Address any feedback or requested changes
- Once approved, your PR will be merged

### After Your PR is Merged

- Delete your feature branch
- Update your local repository:
  ```bash
  git checkout main
  git pull upstream main
  ```

## Development Phases

The project is being developed in phases as outlined in [TODO.md](TODO.md). When contributing:

- Check the current phase to understand priorities
- Align your contributions with the roadmap
- For features not yet in the roadmap, discuss in an issue first

## Questions or Need Help?

- Open an issue for bugs or feature requests
- Start a discussion for questions or ideas
- Check existing issues and documentation first

## License

By contributing to rpview-gpui, you agree that your contributions will be licensed under the same license as the project.

---

Thank you for contributing to rpview-gpui!
