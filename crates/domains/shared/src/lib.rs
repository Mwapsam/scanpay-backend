//! `shared` domain (chunk 1). Specification: Shared draft 0.7 (Claude Docs).
//!
//! Not frozen: no tables, commands or events until this chunk passes its
//! freeze gate (06 §2A). Other domains are reached only through their
//! commands, events and contract views, never their tables.

pub mod db;
pub mod domain;
