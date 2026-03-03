//! This module provides functionality for running Godot with a simple builder pattern.

use crate::cargo::cargo_info_provider;
use crate::command_runner::{Command, CommandChain};
use crate::config::Config;
use crate::download_client::DownloadClient;
use crate::github::GitHubClient;
use crate::godot_version::GodotVersion;
use crate::installer::ensure_installed;
use crate::path_extension::PathExt;
use crate::project_specification::{
    ProjectSpecError, ProjectSpecification, load_godot_project_spec,
};
use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};

pub struct GodotRunner<D: DownloadClient> {
    config: Option<Config>,
    download_client: Option<D>,
    godot_version: Option<GodotVersion>,
    godot_cli_arguments: Option<Vec<String>>,
    godot_project_path: Option<PathBuf>,
    pre_import: bool,
}

impl<D: DownloadClient> GodotRunner<D> {
    /// Run Godot with the current configuration.
    pub async fn build(&self) -> Result<CommandChain> {
        let working_dir = &std::env::current_dir()?;

        let Some(config) = self.config.as_ref() else {
            bail!("A data directory configuration must be specified.");
        };
        let Some(download_client) = self.download_client.as_ref() else {
            bail!("A download client must be specified.");
        };

        let spec_from_file = load_godot_project_spec(working_dir, cargo_info_provider()).or_else(
            |error| -> Result<ProjectSpecification> {
                match error {
                    ProjectSpecError::NotFound => Ok(ProjectSpecification {
                        godot_version: self.godot_version.clone().context(
                            "No Godot version specified and no configuration file found.",
                        )?,
                        godot_project_dir: working_dir.clone(),
                        run_args: vec![],
                        editor_args: vec![],
                        pre_import: true,
                        gdextension: Default::default(),
                        addons: Default::default(),
                    }),
                    _ => bail!(error),
                }
            },
        )?;
        let project_spec = ProjectSpecification {
            godot_version: self
                .godot_version
                .clone()
                .unwrap_or(spec_from_file.godot_version),
            run_args: self
                .godot_cli_arguments
                .clone()
                .unwrap_or(spec_from_file.run_args),
            godot_project_dir: self
                .godot_project_path
                .clone()
                .unwrap_or(spec_from_file.godot_project_dir)
                .to_absolute()?,
            ..spec_from_file
        };

        for (_, generator) in project_spec.gdextension {
            generator.build()?.write()?;
        }

        let executable_path =
            ensure_installed(config, &project_spec.godot_version, download_client, false)
                .await
                .context(format!(
                    "Failed to install Godot version {}",
                    project_spec.godot_version
                ))?;

        let mut command_chain = CommandChain::new();

        if self.pre_import && !project_spec.godot_project_dir.join(".godot").exists() {
            let failure_message = "Possible cause: Known bug in Godot 4.5.1: \"Headless import of project with GDExtensions crashes\"\n\
                See: https://github.com/godotengine/godot/issues/111645\n\
                Try re-running if `.godot` folder was generated successfully.";
            command_chain.append(Command {
                executable: executable_path.clone(),
                working_dir: project_spec.godot_project_dir.clone(),
                args: vec!["--import".to_string(), "--headless".to_string()],
                failure_message: Some(failure_message.to_string()),
            });
        }

        command_chain.append(Command {
            executable: executable_path,
            working_dir: project_spec.godot_project_dir.clone(),
            args: ["--path".to_string(), ".".to_string()]
                .into_iter()
                .chain(project_spec.run_args.iter().cloned())
                .collect(),
            failure_message: None,
        });

        Ok(command_chain)
    }

    pub fn config(self, config: Option<Config>) -> Self {
        Self { config, ..self }
    }

    pub fn download_client(self, download_client: Option<D>) -> Self {
        Self {
            download_client,
            ..self
        }
    }

    pub fn godot_version(self, godot_version: Option<GodotVersion>) -> Self {
        Self {
            godot_version,
            ..self
        }
    }

    pub fn godot_cli_arguments(self, godot_cli_arguments: Option<Vec<String>>) -> Self {
        Self {
            godot_cli_arguments,
            ..self
        }
    }

    pub fn godot_project_path(self, godot_project_path: Option<PathBuf>) -> Self {
        Self {
            godot_project_path,
            ..self
        }
    }

    pub fn pre_import(self, pre_import: bool) -> Self {
        Self { pre_import, ..self }
    }
}

impl<D: DownloadClient> Default for GodotRunner<D> {
    fn default() -> Self {
        Self {
            config: None,
            download_client: None,
            godot_version: None,
            godot_cli_arguments: None,
            godot_project_path: None,
            pre_import: true,
        }
    }
}

impl GodotRunner<GitHubClient> {
    /// Example usage:
    /// ```rust,no_run
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     gdenv_lib::api::godot_runner::GodotRunner::init()?
    ///         .build()
    ///         .await?
    ///         .execute()
    /// }
    /// ```
    pub fn init() -> Result<GodotRunner<GitHubClient>> {
        Self::init_with_custom_data_dir(Some(&Config::default_data_dir()))
    }

    pub fn init_with_custom_data_dir(data_dir: Option<&Path>) -> Result<GodotRunner<GitHubClient>> {
        let config = Config::setup(data_dir)?;
        let github_client = GitHubClient::new(config.clone());
        Ok(GodotRunner {
            config: Some(config),
            download_client: Some(github_client),
            godot_version: None,
            godot_cli_arguments: None,
            godot_project_path: None,
            pre_import: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::installer::get_executable_path;
    use crate::test_helpers::mock_download_client::MockDownloadClient;
    use anyhow::Result;
    use std::path::Path;
    use std::{env, fs};
    use tempdir::TempDir;

    #[tokio::test]
    async fn test_create() -> Result<()> {
        let tmp_dir = TempDir::new("gdenv-lib")?;
        let data_dir = tmp_dir.path().join("data");
        let project_dir = tmp_dir.path().join("project");
        let godot_project_dir = project_dir.join("godot");

        copy_dir_all("test-data/godot_runner_mock_project", &project_dir)
            .context("Failed to copy test data")?;

        env::set_current_dir(&project_dir)?;

        let config = Config::setup(Some(&data_dir))?;
        let runner = GodotRunner::default()
            .config(Some(config.clone()))
            .download_client(Some(MockDownloadClient));

        let command_chain = runner.build().await?;

        assert_eq!(command_chain.commands().len(), 2);
        assert_eq!(
            command_chain.commands()[0].executable.canonicalize()?,
            get_executable_path(&config, &GodotVersion::new("4.2.1-stable", false)?)?
                .canonicalize()?
        );
        assert_eq!(
            command_chain.commands()[0].working_dir.canonicalize()?,
            godot_project_dir.clone().canonicalize()?
        );
        assert_eq!(
            command_chain.commands()[0].args,
            vec!["--import".to_string(), "--headless".to_string()]
        );
        assert_eq!(
            command_chain.commands()[1].executable.canonicalize()?,
            get_executable_path(&config, &GodotVersion::new("4.2.1-stable", false)?)?
                .canonicalize()?
        );
        assert_eq!(
            command_chain.commands()[1].working_dir.canonicalize()?,
            godot_project_dir.clone().canonicalize()?
        );
        assert_eq!(
            command_chain.commands()[1].args,
            vec!["--path".to_string(), ".".to_string()]
        );
        Ok(())
    }

    fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
        fs::create_dir_all(&dst)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let ty = entry.file_type()?;
            if ty.is_dir() {
                copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
            } else {
                fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
            }
        }
        Ok(())
    }
}
