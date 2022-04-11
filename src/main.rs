use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use anyhow::Context;
use clap::Parser;
use rocket::routes;
use tokio::sync::mpsc::unbounded_channel;

mod bot;
use bot::Prololo;

mod config;
use config::ProloloConfig;

mod webhooks;
use webhooks::{
    generic::generic,
    github_webhook,
    prolosite::{django, forum, impersonate, new_school},
    EventSender,
};

#[derive(Parser)]
#[clap(version)]
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

    let prololo = Prololo::new(config.clone()).context("failed to create prololo bot")?;
    prololo.init().await.context("failed to init prololo bot")?;
    tokio::spawn(async move { prololo.run(receiver).await });

    let rocket = rocket::build()
        .mount(
            "/",
            routes![
                github_webhook,
                django,
                forum,
                new_school,
                impersonate,
                generic
            ],
        )
        .manage(EventSender(sender))
        .manage(config);
    rocket.launch().await.map_err(|err| anyhow::anyhow!(err))
}

#[test]
fn check_clap_app() {
    use clap::IntoApp;
    Opts::into_app().debug_assert()
}
