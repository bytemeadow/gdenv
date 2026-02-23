use crate::cli::GlobalArgs;
use crate::commands::run::invoke_godot;
use anyhow::Result;
use clap::Args;

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
        invoke_godot(
            global_args,
            self.version.clone(),
            self.dotnet,
            std::iter::once("--editor".to_string())
                .chain(self.godot_arguments)
                .collect(),
        )
        .await
    }
}
