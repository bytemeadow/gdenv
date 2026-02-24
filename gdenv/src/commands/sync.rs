use crate::cli::GlobalArgs;
use anyhow::Result;
use clap::Args;
use gdenv_lib::addons::sync_addons;
use gdenv_lib::config::Config;
use gdenv_lib::git::SystemGitClient;
use gdenv_lib::project_specification::{ProjectSpecification, load_godot_project_spec};

#[derive(Args)]
pub struct SyncCommand {}

impl SyncCommand {
    pub async fn run(self, global_args: GlobalArgs) -> Result<()> {
        let config = Config::setup(global_args.datadir.as_deref())?;
        let git_client = SystemGitClient::new(config);
        let working_dir = global_args.project.unwrap_or(std::env::current_dir()?);
        let spec_from_file = load_godot_project_spec(&working_dir)?;
        let project_spec = ProjectSpecification { ..spec_from_file };

        sync_addons(project_spec, &working_dir, &git_client).await?;

        Ok(())
    }
}
