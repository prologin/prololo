use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use clap::Clap;

mod bot;
use bot::Prololo;

mod config;
use config::ProloloConfig;

#[derive(Clap)]
#[clap(version = "0.1")]
struct Opts {
    /// File where session information will be saved
    #[clap(short, long, parse(from_os_str))]
    config: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let opts = Opts::parse();
    let config_file = opts.config;
    let config: ProloloConfig = serde_yaml::from_reader(BufReader::new(File::open(config_file)?))?;

    let prololo = Prololo::new(config)?;
    prololo.init().await?;
    prololo.run().await;

    Ok(())
}
