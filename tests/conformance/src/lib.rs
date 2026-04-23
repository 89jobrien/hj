// Conformance test suite — hj workspace boundary contracts.
// Each module maps to one spec section in .ctx/conformance.md.

#[cfg(test)]
mod cli;
#[cfg(test)]
mod core;
#[cfg(test)]
mod doob;
#[cfg(test)]
mod git;
#[cfg(test)]
mod render;
#[cfg(test)]
mod sqlite; // §6 — documents why tests live in hj-cli instead
