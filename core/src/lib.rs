#![no_std]

#[cfg(test)]
extern crate std;

#[path = "../../src/encoder.rs"]
mod encoder;

pub use encoder::*;
