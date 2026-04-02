# ADR 001: Rust + Axum Stack

## Status
Accepted

## Context
Transfer Legacy requires strong safety guarantees, performance, and a mature async stack. The backend must support strict error handling, robust middleware, and long-term maintainability.

## Decision
Use Rust as the primary backend language with Axum + Tokio for the HTTP layer. Use a Cargo workspace with separate crates for API, worker, crypto core, and shared types.

## Consequences
- Strong memory safety and predictable latency.
- Clear separation of concerns and testable components.
- Slightly higher upfront complexity in exchange for long-term safety.
