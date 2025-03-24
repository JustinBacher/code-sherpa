use clap::Parser;

use super::Command;
use crate::prelude::*;

#[derive(Parser, Debug, Clone)]
pub struct Query {
    #[arg(short, long)]
    query: String,
}

impl Command for Query {
    async fn execute(&self) -> Result<()> {
        Ok(())
    }
}
