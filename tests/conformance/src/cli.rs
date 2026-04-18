// §6 — hj-cli composition root contracts
//
// CLI types (Cli, Commands, Args) are pub(crate) in hj-cli and cannot be
// accessed from this external crate. The §6 alias-rewriting and CLI-parsing
// tests live in crates/hj-cli/src/lib.rs#[cfg(test)] where they have full
// access. See s6_* test functions there.
//
// This file is intentionally empty — it documents the decision so the section
// numbering stays consistent with .ctx/conformance.md.
