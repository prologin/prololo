use tokio::sync::mpsc::UnboundedSender;

pub mod github;
pub use github::{github_webhook, GitHubEvent};

pub struct EventSender(pub UnboundedSender<Event>);

#[derive(Debug)]
pub enum Event {
    GitHub(GitHubEvent),
}
