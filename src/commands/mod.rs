mod query;
mod scan;

use clap::{Parser, Subcommand};
use query::Query;
use scan::Scan;

#[derive(Subcommand, Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum Commands {
    Scan(Scan),
    Query(Query),
}

#[derive(Parser, Debug)]
pub struct Args {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,
}

use crate::prelude::*;

pub trait Command {
    async fn execute(&self) -> Result<()>;
}
