//! Asynchronous Single-Reader, Multi-Writer
//!
//! Provides async functions to read data from a single reader, and write it to many writers.

#[macro_use]
extern crate thiserror;

mod copy;
mod flush;
mod seek;
mod validate;

pub use self::{copy::*, validate::*};

use slab::Slab;
use std::ops::{Deref, DerefMut};

/// A multi-writer type which enables copying from a single reader, concurrently to many writers.
pub struct MultiWriter<W> {
    subscribed: Slab<W>,
    drop_list:  Vec<usize>,
}

impl<W> Default for MultiWriter<W> {
    fn default() -> Self { Self { subscribed: Slab::new(), drop_list: Vec::new() } }
}

impl<W> Deref for MultiWriter<W> {
    type Target = Slab<W>;

    fn deref(&self) -> &Self::Target { &self.subscribed }
}

impl<W> DerefMut for MultiWriter<W> {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.subscribed }
}
