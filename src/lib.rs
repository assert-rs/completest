//! Core types for running completions for your program
//!
//! See
//! - completest-pty
//! - completest-nu

#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![warn(clippy::print_stderr)]
#![warn(clippy::print_stdout)]

/// Terminal that shell's will run completions in
#[derive(Debug)]
pub struct Term {
    width: u16,
    height: u16,
}

#[allow(missing_docs)]
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

    pub fn get_width(&self) -> u16 {
        self.width
    }

    pub fn get_height(&self) -> u16 {
        self.height
    }
}

impl Default for Term {
    fn default() -> Self {
        Self::new()
    }
}

/// Abstract factory for [`Runtime`]
pub trait RuntimeBuilder: std::fmt::Debug {
    /// The [`Runtime`] being built
    type Runtime: Runtime;

    /// Name for the runtime (useful for defining a `home`)
    fn name() -> &'static str;

    /// Initialize a new runtime's home
    fn new(
        bin_root: std::path::PathBuf,
        home: std::path::PathBuf,
    ) -> std::io::Result<Self::Runtime>;
    /// Reuse an existing runtime's home
    fn with_home(
        bin_root: std::path::PathBuf,
        home: std::path::PathBuf,
    ) -> std::io::Result<Self::Runtime>;
}

/// Run completions for a shell
pub trait Runtime: std::fmt::Debug {
    /// Location of the runtime's home directory
    fn home(&self) -> &std::path::Path;

    /// Register a completion script
    fn register(&mut self, name: &str, content: &str) -> std::io::Result<()>;

    /// Get the output from typing `input` into the shell
    fn complete(&mut self, input: &str, term: &Term) -> std::io::Result<String>;
}
