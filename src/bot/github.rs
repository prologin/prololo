use std::fmt::Write;

use tracing::error;
use url::Url;

use crate::{
    bot::{message_builder::MessageBuilder, utils::shorten_content, Response},
    webhooks::{
        github::{
            CreateEvent, IssueCommentEvent, IssuesEvent, PullRequestEvent,
            PullRequestReviewCommentEvent, PullRequestReviewEvent, PushEvent, RefType,
        },
        GitHubEvent,
    },
};

const BRANCH: &str = "⊶";
const SEPARATOR: &str = "⋅";
const SHORT_HASH_LENGTH: usize = 7;

pub fn handle_github_event(event: GitHubEvent) -> anyhow::Result<Option<Response>> {
    let response = match event {
        GitHubEvent::Create(event) => handle_create(event),
        GitHubEvent::Issues(event) => handle_issues(event),
        GitHubEvent::IssueComment(event) => handle_issue_comment(event),
        GitHubEvent::Push(event) => handle_push(event),
        GitHubEvent::PullRequest(event) => handle_pull_request(event),
        GitHubEvent::PullRequestReview(event) => handle_pull_request_review(event),
        GitHubEvent::PullRequestReviewComment(event) => handle_pull_request_review_comment(event),
    };

    Ok(response)
}

fn handle_create(event: CreateEvent) -> Option<Response> {
    let mut message = MessageBuilder::new();

    match event.ref_type {
        RefType::Branch => return None,
        RefType::Tag => {
            message.tag(&event.repository.name);

            write!(&mut message, " {} created tag ", event.sender.login,).unwrap();

            let ref_url = match event.repository.ref_url(&event.r#ref) {
                Ok(url) => url,
                Err(e) => {
                    error!(
                        "couldn't build ref url for tag {} in repo {}",
                        event.r#ref, event.repository.full_name
                    );
                    event.repository.html_url
                }
            };
            println!("{}", ref_url);
            message.link(&event.r#ref, ref_url)
        }
    };

    Some(Response {
        message: message,
        repo: Some(event.repository.full_name),
    })
}

fn handle_issues(event: IssuesEvent) -> Option<Response> {
    let action = event.action;
    let issue = event.issue;

    let mut message = MessageBuilder::new();

    message.tag(&event.repository.name);

    write!(&mut message, " {}", event.sender.login).unwrap();

    match action.as_str() {
        "assigned" | "unassigned" => {
            let assignee = event
                .assignee
                .expect("assigned action should always have an assignee");
            let sender = event.sender;
            if assignee.id == sender.id {
                write!(message, " self-{}", action).unwrap();
            } else {
                write!(message, " {} {}", action, assignee.login).unwrap();
            }
            write!(message, " to ").unwrap();
        }

        // too verbose, don't log that
        "labeled" | "unlabeled" => return None,

        "opened" | "deleted" | "pinned" | "unpinned" | "reopened" | "closed" | "locked"
        | "unlocked" | "transferred" => write!(message, " {} issue ", action).unwrap(),

        "edited" => {
            let changes = event
                .changes
                .expect("edited issue without changes shouldn't happen");

            write!(message, " edited").unwrap();
            if changes.title.is_some() {
                write!(message, " title").unwrap();
            }
            if changes.body.is_some() {
                if changes.title.is_some() {
                    write!(message, ",").unwrap();
                }
                write!(message, " body").unwrap();
            }
            write!(message, " of issue ").unwrap();
        }

        "milestoned" => {
            let milestone = issue
                .milestone
                .as_ref()
                .expect("milestoned issue should have a milestone");
            write!(message, " added milestone {} to ", milestone.title).unwrap();
        }

        // https://github.com/isaacs/github/issues/880
        "demilestoned" => write!(message, " removed the milestone from ").unwrap(),

        _ => return None, // FIXME log error
    }

    message.link(&format!("{}", issue), issue.html_url);

    Some(Response {
        message,
        repo: Some(event.repository.full_name),
    })
}

fn handle_issue_comment(event: IssueCommentEvent) -> Option<Response> {
    let action = event.action;
    let comment = event.comment;
    let issue = event.issue;

    // Comments left on PRs are considered as issue comments as well
    let issue_or_pr = match issue.pull_request {
        Some(_) => "PR",
        None => "issue",
    };

    let mut message = MessageBuilder::new();

    message.tag(&event.repository.name);

    write!(&mut message, " {} ", event.sender.login).unwrap();

    match action.as_str() {
        "created" => {
            message.link("commented", comment.html_url);
            write!(message, " on {} ", issue_or_pr,).unwrap();

            message.link(&format!("{}", issue), issue.html_url);

            write!(message, ": {}", shorten_content(&comment.body),).unwrap();
        }

        // too verbose, don't log that
        "edited" | "deleted" => return None,

        _ => return None, // FIXME log error
    }

    Some(Response {
        message,
        repo: Some(event.repository.full_name),
    })
}

fn handle_pull_request(event: PullRequestEvent) -> Option<Response> {
    let action = event.action;
    let pr = event.pull_request;

    let mut message = MessageBuilder::new();

    message.tag(&event.repository.name);

    write!(&mut message, " {}", event.sender.login).unwrap();

    match action.as_str() {
        "assigned" | "unassigned" => {
            let assignee = event
                .assignee
                .expect("assigned action should always have an assignee");
            let sender = event.sender;
            if assignee.id == sender.id {
                write!(message, " self-{}", action).unwrap();
            } else {
                write!(message, " {} {}", action, assignee.login).unwrap();
            }
            write!(message, " to ").unwrap();
            message.link(&format!("{}", pr), pr.html_url);
        }

        "review_requested" => {
            let reviewers = pr
                .requested_reviewers
                .iter()
                .map(|user| user.login.as_str())
                .collect::<Vec<&str>>()
                .join(", ");

            write!(message, " requested {} to review ", reviewers).unwrap();
            message.link(&format!("{}", pr), pr.html_url);
        }

        // too verbose, don't log that
        "labeled" | "unlabeled" | "review_requested_removed" => return None,

        "opened" | "edited" | "reopened" => {
            let base = &pr.base.r#ref;
            let head = &pr.head.r#ref;
            write!(message, " {} ", action).unwrap();
            message.link(&format!("{}", pr), pr.html_url);
            write!(message, " ({}...{})", base, head).unwrap();
        }

        "closed" => {
            let decision = if pr
                .merged
                .expect("PR should always have a merged field in this case")
            {
                "merged"
            } else {
                "closed"
            };
            write!(message, " {} ", decision).unwrap();
            message.link(&format!("{}", pr), pr.html_url);
        }

        _ => return None, // FIXME log error
    }

    Some(Response {
        message,
        repo: Some(event.repository.full_name),
    })
}

fn handle_pull_request_review(event: PullRequestReviewEvent) -> Option<Response> {
    let action = event.action;
    let review = event.review;
    let reviewer = review.user.login;
    let pr = event.pull_request;

    let state = review.state;

    let decision = match state.to_lowercase().as_str() {
        "approved" => "approved",
        "changes_requested" => "requested changes on",
        // FIXME: couldn't find the value of state for comment reviews, find out what it is and make
        //        sure there's a proper error in other cases
        _ => "commented on",
    };

    let mut message = MessageBuilder::new();

    message.tag(&event.repository.name);
    write!(&mut message, " {}", event.sender.login).unwrap();

    match action.as_str() {
        "submitted" => write!(message, " {} {}", decision, pr).unwrap(),

        // ignored, too verbose
        "edited" => return None,

        "dismissed" => {
            write!(message, " dismissed ").unwrap();

            if event.sender.login == reviewer {
                write!(message, "their").unwrap();
            } else {
                write!(message, "{}'s", reviewer).unwrap();
            };

            write!(message, " review for {} (they {} the PR)", pr, decision).unwrap();
        }

        _ => return None, // FIXME log error
    }

    write!(message, " {} {}", SEPARATOR, review.html_url).unwrap();

    Some(Response {
        message,
        repo: Some(event.repository.full_name),
    })
}

fn handle_pull_request_review_comment(event: PullRequestReviewCommentEvent) -> Option<Response> {
    let action = event.action;
    let comment = event.comment;
    let pr = event.pull_request;

    if comment.pull_request_review_id.is_some() {
        // Inline code comment is linked to a PR review, no need to display a message for every
        // comment in that review.
        //
        // Global review event will be handled by the `pull_request_review` event.
        return None;
    }

    let mut message = MessageBuilder::new();

    message.tag(&event.repository.name);

    write!(&mut message, " {}", event.sender.login).unwrap();

    match action.as_str() {
        "created" => {
            write!(message, " commented on {}", pr,).unwrap();

            // comment can be on a specific line of a file
            if let Some(location) = comment.location() {
                write!(message, " {}", location,).unwrap();
            }
        }

        // ignored, too verbose
        "edited" | "deleted" => return None,

        _ => return None, // FIXME log error
    }

    write!(message, " {} {}", SEPARATOR, comment.html_url).unwrap();

    Some(Response {
        message,
        repo: Some(event.repository.full_name),
    })
}

fn handle_push(event: PushEvent) -> Option<Response> {
    let commits = event.commits;

    if commits.is_empty() {
        // no commits => a tag was pushed, handled by `create` events
        return None;
    }

    let pusher = event.sender.login;
    let head = event.head_commit.expect("should have at least one commit");
    // it should be okay to use slicing on a string here because commit hashes should only contain
    // single byte ascii characters
    let hash = &head.id[..SHORT_HASH_LENGTH];
    let force = if event.forced { "force-" } else { "" };

    let mut message = MessageBuilder::new();

    message.tag(&event.repository.name);

    write!(&mut message, " {} {}pushed", pusher, force).unwrap();

    let url: &Url;

    if commits.len() == 1 {
        write!(message, " {}", hash).unwrap();
        url = &head.url;
    } else {
        write!(message, " {} commits", commits.len()).unwrap();

        let distinct_count = commits.iter().filter(|c| c.distinct).count();
        if distinct_count != commits.len() {
            write!(message, " ({} distinct)", distinct_count).unwrap();
        }

        write!(message, " including {}", hash).unwrap();

        url = &event.compare;
    }

    let branch = event
        .r#ref
        .rsplit_once('/')
        .expect("couldn't find branch name")
        .1;

    write!(message, " on").unwrap();
    if event.created {
        write!(message, " new").unwrap();
    }
    write!(
        message,
        " {}{}: {}",
        BRANCH,
        branch,
        shorten_content(head.title())
    )
    .unwrap();

    write!(message, " {} {}", SEPARATOR, url).unwrap();

    Some(Response {
        message,
        repo: Some(event.repository.full_name),
    })
}

#[cfg(test)]
mod tests {
    use crate::webhooks::github::{Comment, GitHubUser, Issue, PrRef, PullRequest, Repository};

    use super::*;

    #[test]
    fn test_handle_create() {
        let event = CreateEvent {
            ref_type: RefType::Tag,
            repository: Repository {
                name: "test-repo".to_string(),
                full_name: "test-user/test-repo".to_string(),
                html_url: Url::parse("https://github.com/test-user/test-repo").unwrap(),
            },
            sender: GitHubUser {
                login: "test-user".to_string(),
                id: 42,
            },
            r#ref: "test-tag".to_string(),
        };

        let response = handle_create(event).expect("should have a response");

        let message = response.message;

        assert_eq!(message.plain, "[test-repo] test-user created tag test-tag",);

        assert_eq!(
            message.html,
            r#"<b>[test-repo]</b> test-user created tag <a href="https://github.com/test-user/test-repo/tree/test-tag">test-tag</a>"#,
        );
    }

    #[test]
    fn test_handle_issues() {
        let event = IssuesEvent {
            repository: Repository {
                name: "test-repo".to_string(),
                full_name: "test-user/test-repo".to_string(),
                html_url: Url::parse("https://github.com/test-user/test-repo").unwrap(),
            },
            sender: GitHubUser {
                login: "test-user".to_string(),
                id: 42,
            },
            issue: Issue {
                number: 42,
                html_url: Url::parse("https://github.com/test-user/test-repo/issues/42").unwrap(),
                title: "Test Issue Title".to_string(),
                milestone: None,
                pull_request: None,
            },
            changes: None,
            assignee: None,
            action: "opened".to_string(),
        };

        let response = handle_issues(event).expect("should have a response");

        let message = response.message;

        assert_eq!(
            message.plain,
            "[test-repo] test-user opened issue #42 (Test Issue Title)",
        );

        assert_eq!(
            message.html,
            r#"<b>[test-repo]</b> test-user opened issue <a href="https://github.com/test-user/test-repo/issues/42">#42 (Test Issue Title)</a>"#,
        );
    }

    #[test]
    fn test_handle_issue_comment() {
        let event = IssueCommentEvent {
            sender: GitHubUser {
                login: "test-user".to_string(),
                id: 42,
            },
            repository: Repository {
                name: "test-repo".to_string(),
                full_name: "test-user/test-repo".to_string(),
                html_url: Url::parse("https://github.com/test-user/test-repo").unwrap(),
            },
            issue: Issue {
                number: 42,
                html_url: Url::parse("https://github.com/test-user/test-repo/issues/42").unwrap(),
                title: "Test Issue Title".to_string(),
                milestone: None,
                pull_request: None,
            },
            action: "created".to_string(),
            comment: Comment {
                html_url: Url::parse("https://github.com/test-user/test-repo/issues/42#issue-42424242").unwrap(),
                body: "This content is very long, longer than our character limit, so it will definitely be truncated".to_string(),
                pull_request_review_id: None,
                path: None,
                position: None,
            },
        };

        let response = handle_issue_comment(event).expect("should have a response");

        let message = response.message;

        assert_eq!(
            message.plain,
            "[test-repo] test-user commented on issue #42 (Test Issue Title): This content is very long, longer than our character limit, so it will d…",
        );

        assert_eq!(
            message.html,
            r#"<b>[test-repo]</b> test-user <a href="https://github.com/test-user/test-repo/issues/42#issue-42424242">commented</a> on issue <a href="https://github.com/test-user/test-repo/issues/42">#42 (Test Issue Title)</a>: This content is very long, longer than our character limit, so it will d…"#,
        );
    }

    #[test]
    fn test_handle_pull_request() {
        let event = PullRequestEvent {
            sender: GitHubUser {
                login: "test-user".to_string(),
                id: 42,
            },
            repository: Repository {
                name: "test-repo".to_string(),
                full_name: "test-user/test-repo".to_string(),
                html_url: Url::parse("https://github.com/test-user/test-repo").unwrap(),
            },
            pull_request: PullRequest {
                number: 42,
                html_url: Url::parse("https://github.com/test-user/test-repo/pull/42").unwrap(),
                title: "Test PR Title".to_string(),
                user: GitHubUser {
                    login: "test-user".to_string(),
                    id: 42,
                },
                requested_reviewers: vec![],
                base: PrRef {
                    r#ref: "main".to_string(),
                },
                head: PrRef {
                    r#ref: "test".to_string(),
                },
                merged: None,
            },
            action: "opened".to_string(),
            assignee: None,
        };

        let response = handle_pull_request(event).expect("should have a response");

        let message = response.message;

        assert_eq!(message.plain, "[test-repo] test-user opened PR #42: Test PR Title by test-user (main...test)",);

        assert_eq!(message.html, r#"<b>[test-repo]</b> test-user opened <a href="https://github.com/test-user/test-repo/pull/42">PR #42: Test PR Title by test-user</a> (main...test)"#,);
    }
}
