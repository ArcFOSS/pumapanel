//! PumaPanel agent.
//!
//! A small daemon that runs Minecraft (PaperMC) servers as containers on the
//! host, using whichever container runtime is present:
//!
//! - on a Docker host it drives Docker,
//! - on a Podman host it drives Podman,
//! - installed directly on the host (bare metal) it prefers Podman.
//!
//! The main API owns server metadata and talks to this agent over a small
//! token-authenticated HTTP API. See [`wire`] for the contract.

pub mod command;
pub mod config;
pub mod detect;
pub mod engine;
pub mod error;
pub mod memory;
pub mod operations;
pub mod routes;
pub mod testing;
pub mod wire;
