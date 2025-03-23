use clap::Parser;

#[derive(Parser, Debug, Clone)]
pub struct Query {
    #[arg(short, long)]
    query: String,
}
