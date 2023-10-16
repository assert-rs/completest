use std::ffi::OsStr;
use std::ffi::OsString;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use nu_cli::NuCompleter;
use nu_command::add_shell_command_context;
use nu_parser::parse;
use nu_protocol::{
    engine::{EngineState, Stack, StateWorkingSet},
    Value,
};
use reedline::Completer;

use crate::build_path;
use crate::Runtime;
use crate::Term;

/// Nushell runtime
///
/// > **WARNING:** This will call `std::env::set_current_dir`
#[derive(Debug)]
#[cfg(feature = "nu")] // purely for rustdoc to pick it up
pub struct NuRuntime {
    path: OsString,
    home: PathBuf,
}

impl NuRuntime {
    pub fn new(bin_root: PathBuf, home: PathBuf) -> std::io::Result<Self> {
        std::fs::create_dir_all(&home)?;

        let config = "";
        let config_path = home.join(".config/nushell/config.nu");
        std::fs::create_dir_all(config_path.parent().unwrap())?;
        std::fs::write(config_path, config)?;

        Self::with_home(bin_root, home)
    }

    pub fn with_home(bin_root: PathBuf, home: PathBuf) -> std::io::Result<Self> {
        let bin_root = dunce::canonicalize(bin_root)?;
        let home = dunce::canonicalize(home)?;
        let path = build_path(bin_root);
        Ok(Self { path, home })
    }

    pub fn home(&self) -> &std::path::Path {
        &self.home
    }

    /// Register a completion script
    pub fn register(&mut self, name: &str, content: &str) -> std::io::Result<()> {
        let path = self
            .home
            .join(format!(".config/nushell/completions/{name}.nu"));
        std::fs::create_dir_all(path.parent().unwrap())?;
        std::fs::write(path, content)
    }

    /// Get the output from typing `input` into the shell
    pub fn complete(&mut self, input: &str, term: &Term) -> std::io::Result<String> {
        use std::fmt::Write as _;

        let input = input.split_once('\t').unwrap_or((input, "")).0;

        let completion_root = self.home.join(".config/nushell/completions");
        let mut completers = std::collections::BTreeMap::new();
        for entry in std::fs::read_dir(&completion_root)? {
            let entry = entry?;
            if let Some(stem) = entry
                .file_name()
                .to_str()
                .unwrap_or_default()
                .strip_suffix(".nu")
            {
                let content = std::fs::read_to_string(entry.path())?;
                completers.insert(stem.to_owned(), content);
            }
        }
        let mut completer = external_completion(&self.path, &self.home, &completers)?;

        let suggestions = completer.complete(input, input.len());

        let mut max_value_len = 0;
        for suggestion in &suggestions {
            max_value_len = suggestion.value.len().max(max_value_len);
        }
        let spacer = "    ";

        let mut buffer = String::new();
        let _ = writeln!(&mut buffer, "% {input}");
        for suggestion in &suggestions {
            let value = &suggestion.value;
            let max_descr_len = (term.width as usize) - max_value_len - spacer.len();
            let descr = suggestion
                .description
                .as_deref()
                .unwrap_or_default()
                .trim_end_matches('\n');
            let spacer = if !descr.is_empty() { spacer } else { "" };
            let descr = &descr[0..max_descr_len.min(descr.len())];
            let _ = writeln!(&mut buffer, "{value}{spacer}{descr}");
        }

        Ok(buffer)
    }
}

impl Runtime for NuRuntime {
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

fn external_completion(
    path: &OsStr,
    home: &Path,
    completers: &std::collections::BTreeMap<String, String>,
) -> std::io::Result<NuCompleter> {
    // Create a new engine
    let (mut engine_state, mut stack) = new_engine(path, home)?;

    for completer in completers.values() {
        let (_, delta) = {
            let mut working_set = StateWorkingSet::new(&engine_state);
            let block = parse(&mut working_set, None, completer.as_bytes(), false);
            if !working_set.parse_errors.is_empty() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    working_set.parse_errors.remove(0),
                ));
            }

            (block, working_set.render())
        };

        engine_state
            .merge_delta(delta)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?;
    }

    // Merge environment into the permanent state
    engine_state
        .merge_env(&mut stack, home)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?;

    if engine_state.num_blocks() == 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "completer not registered",
        ));
    }
    let latest_block_id = engine_state.num_blocks() - 1;

    // Change config adding the external completer
    let mut config = engine_state.get_config().clone();
    config.external_completer = Some(latest_block_id);
    engine_state.set_config(&config);

    // Instantiate a new completer
    Ok(NuCompleter::new(Arc::new(engine_state), stack))
}

/// creates a new engine with the current path into the completions fixtures folder
fn new_engine(path: &OsStr, home: &Path) -> std::io::Result<(EngineState, Stack)> {
    let mut pwd = home
        .to_owned()
        .into_os_string()
        .into_string()
        .unwrap_or_default();
    pwd.push(std::path::MAIN_SEPARATOR);

    let path = path.to_owned().into_string().unwrap_or_default();
    let path_len = path.len();

    // Create a new engine with default context
    let mut engine_state = add_shell_command_context(nu_cmd_lang::create_default_context());

    // New stack
    let mut stack = Stack::new();

    // Add pwd as env var
    stack.add_env_var(
        "PWD".to_string(),
        Value::String {
            val: pwd.clone(),
            internal_span: nu_protocol::Span::new(0, pwd.len()),
        },
    );

    #[cfg(windows)]
    stack.add_env_var(
        "Path".to_string(),
        Value::String {
            val: path,
            internal_span: nu_protocol::Span::new(0, path_len),
        },
    );

    #[cfg(not(windows))]
    stack.add_env_var(
        "PATH".to_string(),
        Value::String {
            val: path,
            internal_span: nu_protocol::Span::new(0, path_len),
        },
    );

    // Merge environment into the permanent state
    engine_state
        .merge_env(&mut stack, home)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?;

    Ok((engine_state, stack))
}
