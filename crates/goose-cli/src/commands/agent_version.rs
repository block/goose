use anyhow::Result;
use clap::Args;
use goose::agents::AgentFactory;
use std::fmt::Write;

#[derive(Args)]
pub struct AgentCommand {}

impl AgentCommand {
    pub fn run(&self) -> Result<()> {
        let mut output = String::new();
        writeln!(output, "Available agent versions:")?;

        let versions = AgentFactory::available_versions();
        let default_version = AgentFactory::default_version();
        let configured_version = AgentFactory::configured_version();

        for version in versions {
            if version == default_version && version == configured_version {
                writeln!(output, "* {} (default)", version)?;
            } else if version == default_version {
                writeln!(output, "  {} (default)", version)?;
            } else if version == configured_version {
                writeln!(output, "* {}", version)?;
            } else {
                writeln!(output, "  {}", version)?;
            }
        }

        print!("{}", output);
        Ok(())
    }
}
