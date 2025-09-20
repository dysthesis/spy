use clap::Parser;
use libspy::{cli::Cli, entry::Entry, template::Template};

#[cfg(all(feature = "dhat-heap", feature = "dhat-ad-hoc"))]
compile_error!("Enable only one of `dhat-heap` or `dhat-ad-hoc` at a time.");

fn main() -> color_eyre::Result<()> {
    #[cfg(feature = "dhat-heap")]
    let _dhat_profiler = dhat::Profiler::new_heap();

    #[cfg(all(feature = "dhat-ad-hoc", not(feature = "dhat-heap")))]
    let _dhat_profiler = dhat::Profiler::new_ad_hoc();

    color_eyre::install()?;

    let cli = Cli::parse();
    // Where we store our data
    let entry = Entry::new(&cli.url, None)?;
    // println!("{}", serde_json::to_string(&entry)?);
    let rendered = cli
        .template
        .map(Template::new)
        .map(|t| t.render(&entry).map_err(color_eyre::Report::from))
        .unwrap_or_else(|| serde_json::to_string(&entry).map_err(color_eyre::Report::from))?;
    println!("{}", rendered);
    Ok(())
}
