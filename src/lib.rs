//! Run completions for your program
//!
//! # Example
//!
//! ```rust,no_run
//! # #[cfg(unix)] {
//! # use std::path::Path;
//! # let bin_root = Path::new("").to_owned();
//! # let completion_script = "";
//! # let home = std::env::current_dir().unwrap();
//! let term = completest::Term::new();
//!
//! let mut runtime = completest::BashRuntime::new(bin_root, home).unwrap();
//! runtime.register("foo", completion_script).unwrap();
//! let output = runtime.complete("foo \t\t", &term).unwrap();
//! # }
//! ```

#![cfg_attr(docsrs, feature(doc_auto_cfg))]

use std::ffi::OsString;
use std::path::PathBuf;

#[cfg(feature = "nu")]
mod nu;
#[cfg(unix)]
mod pty;

#[cfg(feature = "nu")]
pub use nu::*;
#[cfg(unix)]
pub use pty::*;

/// Terminal that shell's will run completions in
#[derive(Debug)]
pub struct Term {
    width: u16,
    height: u16,
}

impl Term {
    pub fn new() -> Self {
        Self {
            width: 120,
            height: 60,
        }
    }

    pub fn width(mut self, width: u16) -> Self {
        self.width = width;
        self
    }

    pub fn height(mut self, height: u16) -> Self {
        self.height = height;
        self
    }
}

impl Default for Term {
    fn default() -> Self {
        Self::new()
    }
}

pub trait Runtime: std::fmt::Debug {
    fn name(&self) -> &'static str;

    fn home(&self) -> &std::path::Path;

    /// Register a completion script
    fn register(&mut self, name: &str, content: &str) -> std::io::Result<()>;

    /// Get the output from typing `input` into the shell
    fn complete(&mut self, input: &str, term: &Term) -> std::io::Result<String>;
}

fn build_path(bin_root: PathBuf) -> OsString {
    let mut path = bin_root.into_os_string();
    if let Some(existing) = std::env::var_os("PATH") {
        path.push(":");
        path.push(existing);
    }
    path
}
