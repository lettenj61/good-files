//! **TODO: Crate level documentation**

/// `file` module is the core of `good-files`, contains
/// convenient wrapper around `std::fs` and `std::io`
/// modules.
pub mod file;

pub use file::File;

pub use file::FileOpener;

pub use file::Open;

pub use file::CreateMode;

pub use file::WriteOption;
