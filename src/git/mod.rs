/*
 * Copyright (c) 2026 Pritam
 *
 * SPDX-License-Identifier: MIT
 */

//! The git actor: `git2::Repository` is `!Sync`, so all access happens on
//! one dedicated thread (`actor.rs`) that owns the repo and processes
//! `RepoCommand`s one at a time. Everything else in the crate talks to it
//! only through the cloneable `RepoHandle`.
//!
//! `ops/` holds one file per git operation (status, log, diff, ...) —
//! split out of what used to be one 392-line `repo.rs` so each operation
//! can be read, tested, and modified independently.

mod actor;
mod commands;
mod handle;
mod ops;

pub use commands::{RepoCommand, RepoResponse};
pub use handle::{RepoHandle, recv_response};
