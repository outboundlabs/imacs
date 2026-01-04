# Contributing to IMACS

Thank you for your interest in contributing to IMACS! This document provides guidelines and instructions for contributing.

## Getting Started

### Prerequisites

- Rust 1.70 or later
- Git

### Development Setup

1. Clone the repository:
   ```bash
   git clone https://github.com/anthropics/imacs.git
   cd imacs
   ```

2. Build the project:
   ```bash
   cargo build
   ```

3. Run tests:
   ```bash
   cargo test
   ```

4. Run the CLI:
   ```bash
   cargo run -- --help
   ```

## Development Workflow

### Running Tests

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run a specific test
cargo test test_name
```

### Code Quality

Before submitting a PR, ensure your code passes all checks:

```bash
# Format code
cargo fmt

# Run clippy
cargo clippy -- -D warnings

# Run tests
cargo test
```

### Building Documentation

```bash
cargo doc --open
```

## Code Style

- Follow standard Rust formatting (`cargo fmt`)
- Use meaningful variable and function names
- Add documentation comments for public APIs
- Keep functions focused and reasonably sized
- Prefer explicit error handling over `.unwrap()`

## Pull Request Process

1. **Fork** the repository and create your branch from `main`
2. **Make your changes** with clear, focused commits
3. **Add tests** for any new functionality
4. **Update documentation** if needed
5. **Run all checks** (`cargo fmt`, `cargo clippy`, `cargo test`)
6. **Submit a PR** with a clear description of changes

### PR Guidelines

- Keep PRs focused on a single feature or fix
- Write clear commit messages
- Reference any related issues
- Be responsive to feedback

## Reporting Issues

### Bug Reports

When reporting bugs, please include:

- IMACS version (`cargo run -- --version`)
- Rust version (`rustc --version`)
- Operating system
- Steps to reproduce
- Expected vs actual behavior
- Relevant spec files or code samples

### Feature Requests

For feature requests, please describe:

- The problem you're trying to solve
- Your proposed solution
- Any alternatives you've considered

## Project Structure

```
imacs/
├── src/
│   ├── lib.rs          # Library entry point
│   ├── main.rs         # CLI entry point
│   ├── spec/           # Spec parsing and validation
│   ├── cel/            # CEL expression handling
│   ├── render/         # Code generation (6 languages)
│   ├── verify/         # Code verification
│   ├── testgen/        # Test generation
│   ├── completeness/   # Completeness analysis
│   ├── orchestrate/    # Workflow orchestration
│   ├── analyze/        # Code analysis
│   ├── extract/        # Spec extraction
│   ├── drift/          # Drift detection
│   └── format/         # Code formatting
├── specs/              # Example specifications
├── examples/           # Usage examples
└── tests/              # Integration tests
```

## License

By contributing to IMACS, you agree that your contributions will be licensed under the MIT License.

## Questions?

If you have questions, feel free to:

- Open an issue for discussion
- Check existing issues and PRs for context

Thank you for contributing!
