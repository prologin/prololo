use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use clap::Clap;
use rocket::routes;

mod bot;
use bot::Prololo;

mod config;
use config::ProloloConfig;

mod webhooks;
use webhooks::github_webhook;

#[derive(Clap)]
#[clap(version = "0.1")]
struct Opts {
    /// Configuration file for prololo
    #[clap(short, long, parse(from_os_str))]
    config: PathBuf,
}

#[rocket::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let opts = Opts::parse();
    let config_file = opts.config;
    let config: ProloloConfig = serde_yaml::from_reader(BufReader::new(File::open(config_file)?))?;

    let prololo = Prololo::new(config)?;
    prololo.init().await?;
    tokio::spawn(async move { prololo.run().await });

    let rocket = rocket::build().mount("/", routes![github_webhook]);
    rocket.launch().await.map_err(|err| anyhow::anyhow!(err))
}
