//! Toolkit to recover user data from a formatted PS3 internal hard drive.
//!
//! The work happens in three stages:
//! 1. [`decrypt`]: turn the encrypted drive image into a readable one.
//! 2. [`carve`]: scan the readable image for known file headers and extract files.
//! 3. [`verify`]: check each extracted file and set aside the broken ones.
//!
//! [`sort`] files a folder of already-recovered files (such as PhotoRec output)
//! into the same category folders that [`carve`] uses.

pub mod carve;
pub mod category;
pub mod decrypt;
pub mod fsutil;
pub mod signatures;
pub mod sort;
pub mod verify;
