//! Reusable settings engine for comment-driven Nix configuration frontends.
//!
//! This crate is the boundary shared with `nix-fleet-manager`. It owns the
//! scanner, model, metadata, value, edit, git, flake, authoring, preview,
//! catalog, and theme layers. It does not own HTTP, PAM, browser sessions, or
//! static assets. Those stay in the `server` crate.

pub mod api;
pub mod paths;
pub mod session;
pub mod theme;

/// Version of the request/response contract exposed to frontends.
pub const API_VERSION: u32 = 1;
