//! `support` domain (chunk 9). Specification: 04 §8.6.
//!
//! Not frozen: no tables, commands or events until this chunk passes its
//! freeze gate (06 §2A). Other domains are reached only through their
//! commands, events and contract views, never their tables.

pub mod db;
pub mod domain;
