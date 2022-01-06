use tokio::sync::mpsc::UnboundedSender;

pub mod github;
pub use github::{github_webhook, GitHubEvent};

pub mod prolosite;
pub(crate) use prolosite::ProloSiteEvent;

pub struct EventSender(pub UnboundedSender<Event>);

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum Event {
    GitHub(GitHubEvent),
    ProloSite(ProloSiteEvent),
}
