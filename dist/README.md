# Distribution notes

The npm artifact contains the complete Rust source adapter, Cargo metadata,
examples, and verification tests. It intentionally excludes `target` and all
platform-specific binaries. Final applications link to an external libpolycall.
