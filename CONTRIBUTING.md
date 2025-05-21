# Contributing to tonic-mock

Thank you for your interest in contributing to tonic-mock! This document provides guidelines and instructions for contributing to this project.

## Code of Conduct

Please be respectful and considerate of others when contributing to this project. We aim to foster an inclusive and welcoming community.

## Getting Started

1. Fork the repository on GitHub
2. Clone your fork locally
3. Add the upstream repository as a remote: `git remote add upstream https://github.com/ORIGINAL_OWNER/tonic-mock.git`
4. Create a new branch for your changes: `git checkout -b feature/your-feature-name`

## Development Environment

To set up your development environment:

1. Install Rust and Cargo (stable channel)
2. Run `cargo build` to build the project
3. Run `cargo test` to run the tests

## Making Changes

When making changes:

1. Ensure your code follows the Rust style guidelines
2. Write tests for new features or bug fixes
3. Update documentation as necessary
4. Run `cargo clippy` to check for linting issues
5. Run `cargo test` to make sure all tests pass

## Commit Guidelines

- Use clear and descriptive commit messages
- Reference issue numbers in commit messages when relevant
- Keep commits focused on a single change

## Pull Request Process

1. Update your fork to latest upstream: `git fetch upstream && git rebase upstream/main`
2. Push your changes to your fork: `git push origin feature/your-feature-name`
3. Create a pull request from your branch to the main branch of the upstream repository
4. Fill out the pull request template with all relevant information
5. Wait for maintainers to review your pull request and address any feedback

## Testing

All code changes should include appropriate tests:

- Unit tests for individual components
- Integration tests for API functionality
- Make sure all existing tests pass with your changes

## Documentation

Please update documentation when:

- Adding new features
- Changing existing functionality
- Fixing bugs that might affect user behavior

Documentation should be clear, concise, and include examples where appropriate.

## Release Process

The release process is handled by the maintainers. If you believe a new release should be made, you can open an issue suggesting a release.

## Questions?

If you have any questions about contributing, please open an issue asking for clarification.

Thank you for contributing to tonic-mock!
