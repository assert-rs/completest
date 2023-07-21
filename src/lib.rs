//! Run completions for your program
//!
//! # Example
//!
//! ```rust,no_run
//! # use std::path::Path;
//! # let bin_root = Path::new("").to_owned();
//! # let completion_script = "";
//! # let home = std::env::current_dir().unwrap();
//! let term = completest::Term::new();
//!
//! let runtime = completest::BashRuntime::new(bin_root, home).unwrap();
//! runtime.register("foo", completion_script).unwrap();
//! let output = runtime.complete("foo \t\t", &term).unwrap();
//! ```

#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![cfg(unix)]

use std::ffi::OsStr;
use std::ffi::OsString;
use std::io::Read as _;
use std::io::Write as _;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use ptyprocess::PtyProcess;

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

pub trait Runtime {
    fn home(&self) -> &std::path::Path;

    /// Register a completion script
    fn register(&self, name: &str, content: &str) -> std::io::Result<()>;

    /// Get the output from typing `input` into the shell
    fn complete(&self, input: &str, term: &Term) -> std::io::Result<String>;
}

/// Runtime-selection of a [`Runtime`] of supported shells
#[derive(Copy, Clone, Debug)]
pub enum Shell {
    Zsh,
    Bash,
    Fish,
    Elvish,
}

impl Shell {
    pub fn init(self, bin_root: PathBuf, home: PathBuf) -> std::io::Result<Box<dyn Runtime>> {
        let runtime: Box<dyn Runtime> = match self {
            Self::Zsh => Box::new(ZshRuntime::new(bin_root, home)?),
            Self::Bash => Box::new(BashRuntime::new(bin_root, home)?),
            Self::Fish => Box::new(FishRuntime::new(bin_root, home)?),
            Self::Elvish => Box::new(ElvishRuntime::new(bin_root, home)?),
        };
        Ok(runtime)
    }

    pub fn with_home(self, bin_root: PathBuf, home: PathBuf) -> Box<dyn Runtime> {
        match self {
            Self::Zsh => Box::new(ZshRuntime::with_home(bin_root, home)),
            Self::Bash => Box::new(BashRuntime::with_home(bin_root, home)),
            Self::Fish => Box::new(FishRuntime::with_home(bin_root, home)),
            Self::Elvish => Box::new(ElvishRuntime::with_home(bin_root, home)),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Zsh => "zsh",
            Self::Bash => "bash",
            Self::Fish => "fish",
            Self::Elvish => "elvish",
        }
    }
}

#[derive(Debug)]
pub struct ZshRuntime {
    path: OsString,
    home: PathBuf,
    timeout: Duration,
}

impl ZshRuntime {
    pub fn new(bin_root: PathBuf, home: PathBuf) -> std::io::Result<Self> {
        std::fs::create_dir_all(&home)?;

        let config_path = home.join(".zshenv");
        let config = "\
fpath=($fpath $ZDOTDIR/zsh)
autoload -U +X compinit && compinit
PS1='%% '
";
        std::fs::write(config_path, config)?;

        Ok(Self::with_home(bin_root, home))
    }

    pub fn with_home(bin_root: PathBuf, home: PathBuf) -> Self {
        let path = build_path(bin_root);

        Self {
            path,
            home,
            timeout: Duration::from_millis(100),
        }
    }

    pub fn home(&self) -> &std::path::Path {
        &self.home
    }

    /// Register a completion script
    pub fn register(&self, name: &str, content: &str) -> std::io::Result<()> {
        let path = self.home.join(format!("zsh/_{name}"));
        std::fs::create_dir_all(path.parent().unwrap())?;
        std::fs::write(path, content)
    }

    /// Get the output from typing `input` into the shell
    pub fn complete(&self, input: &str, term: &Term) -> std::io::Result<String> {
        let mut command = Command::new("zsh");
        command.env("PATH", &self.path).env("ZDOTDIR", &self.home);
        let echo = false;
        comptest(command, echo, input, term, self.timeout)
    }
}

impl Runtime for ZshRuntime {
    fn home(&self) -> &std::path::Path {
        self.home()
    }

    fn register(&self, name: &str, content: &str) -> std::io::Result<()> {
        self.register(name, content)
    }

    fn complete(&self, input: &str, term: &Term) -> std::io::Result<String> {
        self.complete(input, term)
    }
}

#[derive(Debug)]
pub struct BashRuntime {
    path: OsString,
    home: PathBuf,
    config: PathBuf,
    timeout: Duration,
}

impl BashRuntime {
    pub fn new(bin_root: PathBuf, home: PathBuf) -> std::io::Result<Self> {
        std::fs::create_dir_all(&home)?;

        let config_path = home.join(".bashrc");
        let config = "\
PS1='% '
. /etc/bash_completion
"
        .to_string();
        std::fs::write(config_path, config)?;

        Ok(Self::with_home(bin_root, home))
    }

    pub fn with_home(bin_root: PathBuf, home: PathBuf) -> Self {
        let config_path = home.join(".bashrc");
        let path = build_path(bin_root);

        Self {
            path,
            home,
            config: config_path,
            timeout: Duration::from_millis(50),
        }
    }

    pub fn home(&self) -> &std::path::Path {
        &self.home
    }

    /// Register a completion script
    pub fn register(&self, _name: &str, content: &str) -> std::io::Result<()> {
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&self.config)?;
        writeln!(&mut file, "{content}")?;
        Ok(())
    }

    /// Get the output from typing `input` into the shell
    pub fn complete(&self, input: &str, term: &Term) -> std::io::Result<String> {
        let mut command = Command::new("bash");
        command
            .env("PATH", &self.path)
            .args([OsStr::new("--rcfile"), self.config.as_os_str()]);
        let echo = !input.contains("\t\t");
        comptest(command, echo, input, term, self.timeout)
    }
}

impl Runtime for BashRuntime {
    fn home(&self) -> &std::path::Path {
        self.home()
    }

    fn register(&self, name: &str, content: &str) -> std::io::Result<()> {
        self.register(name, content)
    }

    fn complete(&self, input: &str, term: &Term) -> std::io::Result<String> {
        self.complete(input, term)
    }
}

#[derive(Debug)]
pub struct FishRuntime {
    path: OsString,
    home: PathBuf,
    timeout: Duration,
}

impl FishRuntime {
    pub fn new(bin_root: PathBuf, home: PathBuf) -> std::io::Result<Self> {
        std::fs::create_dir_all(&home)?;

        let config_path = home.join("fish/config.fish");
        let config = "\
set -U fish_greeting \"\"
function fish_title
end
function fish_prompt
    printf '%% '
end;
"
        .to_string();
        std::fs::create_dir_all(config_path.parent().unwrap())?;
        std::fs::write(config_path, config)?;

        Ok(Self::with_home(bin_root, home))
    }

    pub fn with_home(bin_root: PathBuf, home: PathBuf) -> Self {
        let path = build_path(bin_root);

        Self {
            path,
            home,
            timeout: Duration::from_millis(50),
        }
    }

    pub fn home(&self) -> &std::path::Path {
        &self.home
    }

    /// Register a completion script
    pub fn register(&self, name: &str, content: &str) -> std::io::Result<()> {
        let path = self.home.join(format!("fish/completions/{name}.fish"));
        std::fs::create_dir_all(path.parent().unwrap())?;
        std::fs::write(path, content)
    }

    /// Get the output from typing `input` into the shell
    pub fn complete(&self, input: &str, term: &Term) -> std::io::Result<String> {
        let mut command = Command::new("fish");
        command
            .env("PATH", &self.path)
            .env("XDG_CONFIG_HOME", &self.home);
        let echo = false;
        comptest(command, echo, input, term, self.timeout)
    }
}

impl Runtime for FishRuntime {
    fn home(&self) -> &std::path::Path {
        self.home()
    }

    fn register(&self, name: &str, content: &str) -> std::io::Result<()> {
        self.register(name, content)
    }

    fn complete(&self, input: &str, term: &Term) -> std::io::Result<String> {
        self.complete(input, term)
    }
}

#[derive(Debug)]
pub struct ElvishRuntime {
    path: OsString,
    home: PathBuf,
    config: PathBuf,
    timeout: Duration,
}

impl ElvishRuntime {
    pub fn new(bin_root: PathBuf, home: PathBuf) -> std::io::Result<Self> {
        std::fs::create_dir_all(&home)?;

        let config_path = home.join("elvish/rc.elv");
        let config = "\
set edit:rprompt = (constantly \"\")
set edit:prompt = (constantly \"% \")
"
        .to_string();
        std::fs::create_dir_all(config_path.parent().unwrap())?;
        std::fs::write(config_path, config)?;

        Ok(Self::with_home(bin_root, home))
    }

    pub fn with_home(bin_root: PathBuf, home: PathBuf) -> Self {
        let config_path = home.join("elvish/rc.elv");
        let path = build_path(bin_root);

        Self {
            path,
            home,
            config: config_path,
            timeout: Duration::from_millis(50),
        }
    }

    pub fn home(&self) -> &std::path::Path {
        &self.home
    }

    /// Register a completion script
    pub fn register(&self, _name: &str, content: &str) -> std::io::Result<()> {
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&self.config)?;
        writeln!(&mut file, "{content}")?;
        Ok(())
    }

    /// Get the output from typing `input` into the shell
    pub fn complete(&self, input: &str, term: &Term) -> std::io::Result<String> {
        let mut command = Command::new("elvish");
        command
            .env("PATH", &self.path)
            .env("XDG_CONFIG_HOME", &self.home);
        let echo = false;
        comptest(command, echo, input, term, self.timeout)
    }
}

impl Runtime for ElvishRuntime {
    fn home(&self) -> &std::path::Path {
        self.home()
    }

    fn register(&self, name: &str, content: &str) -> std::io::Result<()> {
        self.register(name, content)
    }

    fn complete(&self, input: &str, term: &Term) -> std::io::Result<String> {
        self.complete(input, term)
    }
}

fn build_path(bin_root: PathBuf) -> OsString {
    let mut path = bin_root.into_os_string();
    if let Some(existing) = std::env::var_os("PATH") {
        path.push(":");
        path.push(existing);
    }
    path
}

fn comptest(
    command: Command,
    echo: bool,
    input: &str,
    term: &Term,
    timeout: Duration,
) -> std::io::Result<String> {
    // spawn a new process, pass it the input was.
    //
    // This triggers completion loading process which takes some time in shell so we should let it
    // run for some time
    let mut process = PtyProcess::spawn(command)?;
    process.set_window_size(term.width, term.height)?;
    // for some reason bash does not produce anything with echo disabled...
    process.set_echo(echo, None)?;

    let mut parser = vt100::Parser::new(term.height, term.width, 0);

    let mut stream = process.get_raw_handle()?;
    // pass the completion input
    write!(stream, "{}", input)?;
    stream.flush()?;

    let (snd, rcv) = std::sync::mpsc::channel();

    std::thread::spawn(move || {
        // since we don't know when exactly shell is done completing the idea is to wait until
        // something at all is produced, then wait for some duration since the last produced chunk.
        rcv.recv().unwrap();
        loop {
            std::thread::sleep(timeout);
            let mut cnt = 0;
            while rcv.try_recv().is_ok() {
                cnt += 1;
            }
            if cnt == 0 {
                break;
            }
        }
        process.exit(false).unwrap();
    });
    let mut buf = [0; 2048];

    while let Ok(n) = stream.read(&mut buf) {
        let buf = &buf[..n];
        if buf.is_empty() {
            break;
        }
        snd.send(())
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?;
        parser.process(buf);
    }
    Ok(parser.screen().contents())
}
