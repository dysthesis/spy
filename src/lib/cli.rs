use clap::Parser;
use url::Url;
#[derive(Parser, Debug)]
#[command(
    version,
    name = "spy",
    about = "Fetch information on a webpage on the command line"
)]
pub struct Cli {
    /// The bookmark to the URL.
    pub url: Url,

    #[arg(short = 't', long)]
    /// Optional template string. If none is passed in, it will be serialised as JSON.
    pub template: Option<String>,
}
