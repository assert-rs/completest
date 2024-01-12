use std::ffi::OsStr;
use std::ffi::OsString;
use std::io::Read as _;
use std::io::Write as _;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use ptyprocess::PtyProcess;

use crate::build_path;
use crate::Runtime;
use crate::Term;

#[derive(Debug)]
#[cfg(unix)] // purely for rustdoc to pick it up
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
precmd_functions=\"\"  # avoid the prompt being overwritten
PS1='%% '
PROMPT='%% '
";
        std::fs::write(config_path, config)?;

        let _ = std::fs::remove_file(home.join(".zcompdump"));

        Self::with_home(bin_root, home)
    }

    pub fn with_home(bin_root: PathBuf, home: PathBuf) -> std::io::Result<Self> {
        let path = build_path(bin_root);

        Ok(Self {
            path,
            home,
            timeout: Duration::from_millis(100),
        })
    }

    pub fn home(&self) -> &std::path::Path {
        &self.home
    }

    /// Register a completion script
    pub fn register(&mut self, name: &str, content: &str) -> std::io::Result<()> {
        let path = self.home.join(format!("zsh/_{name}"));
        std::fs::create_dir_all(path.parent().unwrap())?;
        std::fs::write(path, content)
    }

    /// Get the output from typing `input` into the shell
    pub fn complete(&mut self, input: &str, term: &Term) -> std::io::Result<String> {
        let mut command = Command::new("zsh");
        command.arg("--noglobalrcs");
        command.env("PATH", &self.path).env("ZDOTDIR", &self.home);
        let echo = false;
        comptest(command, echo, input, term, self.timeout)
    }
}

impl Runtime for ZshRuntime {
    fn name(&self) -> &'static str {
        "zsh"
    }

    fn home(&self) -> &std::path::Path {
        self.home()
    }

    fn register(&mut self, name: &str, content: &str) -> std::io::Result<()> {
        self.register(name, content)
    }

    fn complete(&mut self, input: &str, term: &Term) -> std::io::Result<String> {
        self.complete(input, term)
    }
}

#[derive(Debug)]
#[cfg(unix)] // purely for rustdoc to pick it up
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

        Self::with_home(bin_root, home)
    }

    pub fn with_home(bin_root: PathBuf, home: PathBuf) -> std::io::Result<Self> {
        let config_path = home.join(".bashrc");
        let path = build_path(bin_root);

        Ok(Self {
            path,
            home,
            config: config_path,
            timeout: Duration::from_millis(50),
        })
    }

    pub fn home(&self) -> &std::path::Path {
        &self.home
    }

    /// Register a completion script
    pub fn register(&mut self, _name: &str, content: &str) -> std::io::Result<()> {
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&self.config)?;
        writeln!(&mut file, "{content}")?;
        Ok(())
    }

    /// Get the output from typing `input` into the shell
    pub fn complete(&mut self, input: &str, term: &Term) -> std::io::Result<String> {
        let mut command = Command::new("bash");
        command
            .env("PATH", &self.path)
            .args([OsStr::new("--rcfile"), self.config.as_os_str()]);
        let echo = !input.contains("\t\t");
        comptest(command, echo, input, term, self.timeout)
    }
}

impl Runtime for BashRuntime {
    fn name(&self) -> &'static str {
        "bash"
    }

    fn home(&self) -> &std::path::Path {
        self.home()
    }

    fn register(&mut self, name: &str, content: &str) -> std::io::Result<()> {
        self.register(name, content)
    }

    fn complete(&mut self, input: &str, term: &Term) -> std::io::Result<String> {
        self.complete(input, term)
    }
}

#[derive(Debug)]
#[cfg(unix)] // purely for rustdoc to pick it up
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
set -U fish_autosuggestion_enabled 0
function fish_title
end
function fish_prompt
    printf '%% '
end;
"
        .to_string();
        std::fs::create_dir_all(config_path.parent().unwrap())?;
        std::fs::write(config_path, config)?;

        Self::with_home(bin_root, home)
    }

    pub fn with_home(bin_root: PathBuf, home: PathBuf) -> std::io::Result<Self> {
        let path = build_path(bin_root);

        Ok(Self {
            path,
            home,
            timeout: Duration::from_millis(50),
        })
    }

    pub fn home(&self) -> &std::path::Path {
        &self.home
    }

    /// Register a completion script
    pub fn register(&mut self, name: &str, content: &str) -> std::io::Result<()> {
        let path = self.home.join(format!("fish/completions/{name}.fish"));
        std::fs::create_dir_all(path.parent().unwrap())?;
        std::fs::write(path, content)
    }

    /// Get the output from typing `input` into the shell
    pub fn complete(&mut self, input: &str, term: &Term) -> std::io::Result<String> {
        let mut command = Command::new("fish");
        command
            .env("PATH", &self.path)
            .env("XDG_CONFIG_HOME", &self.home);
        let echo = false;
        comptest(command, echo, input, term, self.timeout)
    }
}

impl Runtime for FishRuntime {
    fn name(&self) -> &'static str {
        "fish"
    }

    fn home(&self) -> &std::path::Path {
        self.home()
    }

    fn register(&mut self, name: &str, content: &str) -> std::io::Result<()> {
        self.register(name, content)
    }

    fn complete(&mut self, input: &str, term: &Term) -> std::io::Result<String> {
        self.complete(input, term)
    }
}

#[derive(Debug)]
#[cfg(unix)] // purely for rustdoc to pick it up
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

        Self::with_home(bin_root, home)
    }

    pub fn with_home(bin_root: PathBuf, home: PathBuf) -> std::io::Result<Self> {
        let config_path = home.join("elvish/rc.elv");
        let path = build_path(bin_root);

        Ok(Self {
            path,
            home,
            config: config_path,
            timeout: Duration::from_millis(50),
        })
    }

    pub fn home(&self) -> &std::path::Path {
        &self.home
    }

    /// Register a completion script
    pub fn register(&mut self, _name: &str, content: &str) -> std::io::Result<()> {
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&self.config)?;
        writeln!(&mut file, "{content}")?;
        Ok(())
    }

    /// Get the output from typing `input` into the shell
    pub fn complete(&mut self, input: &str, term: &Term) -> std::io::Result<String> {
        let mut command = Command::new("elvish");
        command
            .env("PATH", &self.path)
            .env("XDG_CONFIG_HOME", &self.home);
        let echo = false;
        comptest(command, echo, input, term, self.timeout)
    }
}

impl Runtime for ElvishRuntime {
    fn name(&self) -> &'static str {
        "elvish"
    }

    fn home(&self) -> &std::path::Path {
        self.home()
    }

    fn register(&mut self, name: &str, content: &str) -> std::io::Result<()> {
        self.register(name, content)
    }

    fn complete(&mut self, input: &str, term: &Term) -> std::io::Result<String> {
        self.complete(input, term)
    }
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

    let shutdown = std::sync::atomic::AtomicBool::new(false);
    let shutdown_ref = &shutdown;
    std::thread::scope(|scope| {
        scope.spawn(move || {
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
            shutdown_ref.store(true, std::sync::atomic::Ordering::SeqCst);
            process.exit(false).unwrap();
        });

        let mut buf = [0; 2048];
        while let Ok(n) = stream.read(&mut buf) {
            if shutdown.load(std::sync::atomic::Ordering::SeqCst) {
                // fish clears completions on process teardown
                break;
            }
            let buf = &buf[..n];
            if buf.is_empty() {
                break;
            }
            let _ = snd.send(());
            parser.process(buf);
        }
    });

    let content = parser.screen().contents();
    let content = content
        .lines()
        .map(|l| l.trim())
        .collect::<Vec<_>>()
        .join("\n");
    Ok(content)
}
