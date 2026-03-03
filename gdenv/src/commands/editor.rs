use crate::cli::GlobalArgs;
use anyhow::Result;
use clap::Args;
use gdenv_lib::api::godot_runner::GodotRunner;
use gdenv_lib::godot_version::GodotVersion;

#[derive(Args)]
pub struct EditorCommand {
    /// Override the Godot version for this run
    #[arg(long)]
    pub version: Option<String>,

    /// Use the .NET version
    #[arg(long, alias = "mono")]
    pub dotnet: bool,

    /// Arguments to pass to Godot
    #[arg(last = true)]
    godot_arguments: Vec<String>,
}

impl EditorCommand {
    pub async fn run(self, global_args: GlobalArgs) -> Result<()> {
        GodotRunner::init_with_custom_data_dir(global_args.datadir.as_deref())?
            .godot_cli_arguments(Some(
                std::iter::once("--editor".to_string())
                    .chain(self.godot_arguments)
                    .collect(),
            ))
            .godot_version(
                self.version
                    .map(|v| GodotVersion::new(&v, self.dotnet))
                    .transpose()?,
            )
            .build()?
            .execute()
    }
}
