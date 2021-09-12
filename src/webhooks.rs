mod github;
pub use github::{github_webhook, GitHubEvent};
use tokio::sync::mpsc::UnboundedSender;

pub struct EventSender(pub UnboundedSender<Event>);

#[derive(Debug)]
pub enum Event {
    GitHub(GitHubEvent),
}
