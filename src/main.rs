use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use anyhow::Context;
use clap::Clap;
use rocket::routes;
use tokio::sync::mpsc::unbounded_channel;

mod bot;
use bot::Prololo;

mod config;
use config::ProloloConfig;

mod webhooks;
use webhooks::{
    github::GitHubSecret,
    github_webhook,
    prolosite::{django, forum, new_school, ProlositeSecret},
    EventSender,
};

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
    let config_file = File::open(&opts.config)
        .with_context(|| format!("couldn't open {}:", opts.config.display()))?;
    let config: ProloloConfig = serde_yaml::from_reader(BufReader::new(config_file))
        .context("couldn't parse config file")?;

    let (sender, receiver) = unbounded_channel();
    let github_secret = config.github_secret.clone();
    let prolosite_secret = config.prolosite_secret.clone();

    let prololo = Prololo::new(config).context("failed to create prololo bot")?;
    prololo.init().await.context("failed to init prololo bot")?;
    tokio::spawn(async move { prololo.run(receiver).await });

    let rocket = rocket::build()
        .mount("/", routes![github_webhook, django, forum, new_school])
        .manage(EventSender(sender))
        .manage(GitHubSecret(github_secret))
        .manage(ProlositeSecret(prolosite_secret));
    rocket.launch().await.map_err(|err| anyhow::anyhow!(err))
}
