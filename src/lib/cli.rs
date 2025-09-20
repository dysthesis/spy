use clap::{Parser, Subcommand};
use url::Url;
#[derive(Parser, Debug)]
#[command(
    version,
    name = "spy",
    about = "Fetch information on a webpage on the command line"
)]
pub struct Cli {
    #[command(subcommand)]
    pub subcommand: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    Add {
        /// The bookmark to the URL.
        url: Url,

        /// What tags to add to the bookmark
        #[arg(short = 't', long)]
        tags: Vec<String>,

        /// Optional explicit name for the entry. If not, provided, we will attempt to fetch the
        /// metadata from the page.
        #[arg(short = 'T', long)]
        title: Option<String>,
    },
}
