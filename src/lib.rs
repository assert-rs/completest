//! Core types for running completions for your program
//!
//! See
//! - completest-pty
//! - completest-nu

#![cfg_attr(docsrs, feature(doc_auto_cfg))]

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

pub trait RuntimeBuilder: std::fmt::Debug {
    type Runtime: Runtime;

    fn name() -> &'static str;

    fn new(
        bin_root: std::path::PathBuf,
        home: std::path::PathBuf,
    ) -> std::io::Result<Self::Runtime>;
    fn with_home(
        bin_root: std::path::PathBuf,
        home: std::path::PathBuf,
    ) -> std::io::Result<Self::Runtime>;
}

pub trait Runtime: std::fmt::Debug {
    fn home(&self) -> &std::path::Path;

    /// Register a completion script
    fn register(&mut self, name: &str, content: &str) -> std::io::Result<()>;

    /// Get the output from typing `input` into the shell
    fn complete(&mut self, input: &str, term: &Term) -> std::io::Result<String>;
}
