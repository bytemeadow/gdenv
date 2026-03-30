use crate::cli::GlobalArgs;
use anyhow::Result;
use clap::Args;
use gdenv_lib::project_specification::spec_documentation;

#[derive(Args)]
pub struct SchemaCommand {}

impl SchemaCommand {
    pub async fn run(self, _global_args: GlobalArgs) -> Result<()> {
        println!("{}", spec_documentation()?);
        Ok(())
    }
}
