# tonic-mock Documentation

This directory contains documentation for the tonic-mock library.

## Contents

- [Tutorial](tutorial.md) - A comprehensive tutorial on using tonic-mock for testing gRPC services

## Overview

tonic-mock is a library for testing gRPC services built with the tonic framework in Rust. It provides utilities to make testing streaming interfaces, error handling, and other complex gRPC patterns easier.

## Key Features

- Support for testing all gRPC communication patterns:
  - Unary RPC (one request, one response)
  - Client streaming (multiple requests, one response)
  - Server streaming (one request, multiple responses)
  - Bidirectional streaming (multiple requests, multiple responses)
- Utilities for handling errors and testing error conditions
- Timeout support for streaming operations
- Request interceptors for adding metadata and headers
- Test utilities for creating test messages and responses

## Getting Started

To get started with tonic-mock, check out the [Tutorial](tutorial.md) for detailed examples and patterns for testing different types of gRPC services.

## Contributing

If you'd like to contribute to the documentation, please read the [Contributing Guidelines](../CONTRIBUTING.md) and submit a pull request.
