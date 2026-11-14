//! Reusable settings engine for comment-driven Nix configuration frontends.
//!
//! This crate is the boundary shared with `nix-fleet-manager`. It owns the
//! scanner, model, metadata, value, edit, git, flake, authoring, preview,
//! catalog, and theme layers. It does not own HTTP, PAM, browser sessions, or
//! static assets. Those stay in the `server` crate.

pub mod api;
pub mod apply;
pub mod authoring;
pub mod catalog;
pub mod config;
pub mod flake;
pub mod git;
pub mod paths;
pub mod preview;
pub mod render;
pub mod secureboot;
pub mod session;
pub mod structure;
pub mod users;
pub mod theme;

/// Version of the request/response contract exposed to frontends.
pub const API_VERSION: u32 = 1;
