use crate::cli::GlobalArgs;
use crate::ui;
use anyhow::Result;
use clap::Args;
use gdenv_lib::addons::sync_addons;
use gdenv_lib::cargo::cargo_info_provider;
use gdenv_lib::config::Config;
use gdenv_lib::git::SystemGitClient;
use gdenv_lib::project_specification::{ProjectSpecification, load_godot_project_spec};
use std::io;
use std::io::Write;

#[derive(Args)]
pub struct SyncCommand {
    /// Do not ask for confirmation before syncing
    #[arg(long, short)]
    pub yes: bool,
}

impl SyncCommand {
    pub async fn run(self, global_args: GlobalArgs) -> Result<()> {
        let config = Config::setup(global_args.datadir.as_deref())?;
        let git_client = SystemGitClient::new(config);
        let working_dir = global_args.project.unwrap_or(std::env::current_dir()?);
        let spec_from_file = load_godot_project_spec(&working_dir, cargo_info_provider())?;
        let project_spec = ProjectSpecification { ..spec_from_file };

        if !self.yes {
            ui::warning("Warning! Synchronizing addons is a potentially destructive operation.");
            ui::warning(
                "In order to sync, files that do not exist in the addon must be removed, which could result in the loss of data.",
            );
            ui::warning(
                "Please make sure you have commited your changes and/or created a backup of your project.",
            );
            ui::warning("Any manual changes to addons will be lost.");
            ui::question("Are you ready to synchronize? [y/N]: ");
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

            let confirmed = input.trim().to_lowercase();
            if confirmed != "y" && confirmed != "yes" {
                ui::warning("Synchronization cancelled.");
                return Ok(());
            }
        }

        sync_addons(project_spec, &working_dir, &git_client).await?;

        Ok(())
    }
}
