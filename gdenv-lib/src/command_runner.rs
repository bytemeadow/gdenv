//! Utilities for executing external programs in structures that are easy to test.

use anyhow::Result;
use anyhow::{Context, bail};
use std::fmt::{Display, Formatter};
use std::path::PathBuf;

pub struct Command {
    pub executable: PathBuf,
    pub working_dir: PathBuf,
    pub args: Vec<String>,
    pub failure_message: Option<String>,
}

#[derive(Default)]
pub struct CommandChain {
    commands: Vec<Command>,
}

impl Command {
    fn execute(&self) -> Result<()> {
        let mut command = std::process::Command::new(&self.executable);

        let status = command
            .current_dir(&self.working_dir)
            .args(&self.args)
            .spawn()
            .with_context(|| format!("Failed to spawn process: {:?}", command))?
            .wait()
            .with_context(|| format!("Failed to wait for process: {:?}", command))?;

        if !status.success() {
            let message = self
                .failure_message
                .as_ref()
                .map(|m| format!("\n{}", m))
                .unwrap_or_default();
            bail!(
                "Process exited with code {}\nCommand: {}{}",
                status,
                self,
                message
            )
        } else {
            Ok(())
        }
    }
}

impl CommandChain {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a new command to the chain.
    pub fn append(&mut self, command: Command) -> &mut Self {
        self.commands.push(command);
        self
    }

    /// Execute commands in sequence. If any command fails, the entire chain fails immediately.
    pub fn execute(&self) -> Result<()> {
        for command in self.commands.iter() {
            command.execute()?;
        }
        Ok(())
    }

    pub fn commands(&self) -> &[Command] {
        &self.commands
    }
}

impl Display for Command {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            r#"cd "{}" && {}"#,
            self.working_dir.display(),
            self.executable.display()
        )?;
        for arg in &self.args {
            write!(f, " {}", arg)?;
        }
        Ok(())
    }
}

impl Display for CommandChain {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut first = true;
        for command in &self.commands {
            if !first {
                write!(f, " && ")?;
            }
            write!(f, "{}", command)?;
            first = false;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_chain_display() {
        let chain = CommandChain {
            commands: vec![
                Command {
                    executable: PathBuf::from("echo"),
                    working_dir: PathBuf::from("/home/user"),
                    args: vec![String::from("hello")],
                    failure_message: None,
                },
                Command {
                    executable: PathBuf::from("cat"),
                    working_dir: PathBuf::from("/home/user"),
                    args: vec![String::from("world")],
                    failure_message: None,
                },
            ],
        };
        assert_eq!(
            format!("{}", chain),
            r#"cd "/home/user" && echo hello && cd "/home/user" && cat world"#
        );
    }
}
