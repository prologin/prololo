use std::fmt::Write;

use tracing::{error, info};
use url::Url;

use crate::{
    bot::{emoji, message_builder::MessageBuilder, utils::shorten_content, Response},
    webhooks::{
        github::{
            CreateEvent, IssueCommentEvent, IssuesEvent, OrganizationEvent, PingEvent,
            PullRequestEvent, PullRequestReviewCommentEvent, PullRequestReviewEvent, PushEvent,
            RefType, RepositoryEvent,
        },
        GitHubEvent,
    },
};

const BRANCH: &str = "⊶";
const SHORT_HASH_LENGTH: usize = 7;

pub fn handle_github_event(event: GitHubEvent) -> anyhow::Result<Option<Response>> {
    let response = match event {
        GitHubEvent::CommitComment(event) => handle_commit_comment(event),
        GitHubEvent::Create(event) => handle_create(event),
        GitHubEvent::Fork(event) => handle_fork(event),
        GitHubEvent::IssueComment(event) => handle_issue_comment(event),
        GitHubEvent::Issues(event) => handle_issues(event),
        GitHubEvent::Membership(event) => handle_membership(event),
        GitHubEvent::Organization(event) => handle_organization(event),
        GitHubEvent::Ping(event) => handle_ping(event),
        GitHubEvent::PullRequest(event) => handle_pull_request(event),
        GitHubEvent::PullRequestReview(event) => handle_pull_request_review(event),
        GitHubEvent::PullRequestReviewComment(event) => handle_pull_request_review_comment(event),
        GitHubEvent::Push(event) => handle_push(event),
        GitHubEvent::Repository(event) => handle_repository(event),
    };

    Ok(response)
}

fn handle_commit_comment(event: crate::webhooks::github::CommitCommentEvent) -> Option<Response> {
    let comment = event.comment;
    let commit_id = comment
        .commit_id
        .expect("commit comment without a commit id");

    let mut commit_html_url = comment.html_url.clone();
    commit_html_url.set_fragment(None);

    let mut message = MessageBuilder::new();

    message.tag(&event.repository.name, Some(emoji::SPEECH_BALLOON));

    write!(&mut message, " {} ", event.sender.login).unwrap();

    message.main_link("commented", &comment.html_url);
    write!(message, " on ").unwrap();
    message.link(&commit_id[..SHORT_HASH_LENGTH], &commit_html_url);

    write!(message, ": {}", shorten_content(&comment.body)).unwrap();

    Some(Response {
        message,
        repo: Some(event.repository.full_name),
    })
}

fn handle_create(event: CreateEvent) -> Option<Response> {
    let mut message = MessageBuilder::new();

    match event.ref_type {
        RefType::Branch => return None,
        RefType::Tag => {
            message.tag(&event.repository.name, None);

            write!(&mut message, " {} created tag ", event.sender.login,).unwrap();

            let ref_url = match event.repository.ref_url(&event.r#ref) {
                Ok(url) => url,
                Err(e) => {
                    error!(
                        "couldn't build ref url for tag {} in repo {}: {}",
                        event.r#ref, event.repository.full_name, e
                    );
                    event.repository.html_url
                }
            };
            message.main_link(&event.r#ref, &ref_url)
        }
    };

    Some(Response {
        message,
        repo: Some(event.repository.full_name),
    })
}

fn handle_fork(event: crate::webhooks::github::ForkEvent) -> Option<Response> {
    let mut message = MessageBuilder::new();

    message.tag(&event.repository.name, Some(emoji::PACKAGE));
    write!(&mut message, " ").unwrap();
    message.link(&event.sender.login, &event.sender.html_url);
    write!(&mut message, " forked into ").unwrap();
    message.main_link(&event.forkee.full_name, &event.forkee.html_url);

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

    message.tag(&event.repository.name, Some(emoji::WRENCH));

    write!(&mut message, " {} ", event.sender.login).unwrap();

    match action.as_str() {
        "created" => {
            message.main_link("commented", &comment.html_url);
            write!(message, " on {} ", issue_or_pr,).unwrap();

            message.link(&format!("{}", issue), &issue.html_url);

            write!(message, ": {}", shorten_content(&comment.body),).unwrap();
        }

        // too verbose, don't log that
        "edited" | "deleted" => return None,

        _ => {
            error!("invalid or unsupported issue comment action: {}", action);
            return None;
        }
    }

    Some(Response {
        message,
        repo: Some(event.repository.full_name),
    })
}

fn handle_issues(event: IssuesEvent) -> Option<Response> {
    let action = event.action;
    let issue = event.issue;

    let mut message = MessageBuilder::new();

    message.tag(&event.repository.name, Some(emoji::WRENCH));

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
            if changes.title.is_some() && changes.body.is_some() {
                write!(message, " title and body of issue ").unwrap();
            } else if changes.title.is_some() {
                write!(message, " title of issue ").unwrap();
            } else if changes.body.is_some() {
                write!(message, " body of issue ").unwrap();
            } else {
                error!("issue was edited but received an empty change!");
                return None;
            }
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

        _ => {
            error!("invalid or unsupported issues action: {}", action);
            return None;
        }
    }

    message.main_link(&format!("{}", issue), &issue.html_url);

    Some(Response {
        message,
        repo: Some(event.repository.full_name),
    })
}

fn handle_membership(event: crate::webhooks::github::MembershipEvent) -> Option<Response> {
    let action = event.action;

    let mut message = MessageBuilder::new();

    message.tag(&event.team.name, Some(emoji::PEOPLE));

    // Do not leak secret teams
    if event.team.privacy == "secret" {
        info!("Team {} is secret, not sending anything", event.team.name);
        return None;
    }

    let preposition = match action.as_str() {
        "added" => "to",
        "removed" => "from",
        _ => {
            error!("invalid or unsupported membership action: {}", action);
            return None;
        }
    };

    write!(&mut message, " {} {} ", event.sender.login, action).unwrap();
    message.link(&event.member.login, &event.member.html_url);
    write!(&mut message, " {} the team", preposition).unwrap();

    Some(Response {
        message,
        repo: None,
    })
}

fn handle_organization(event: OrganizationEvent) -> Option<Response> {
    let action = event.action;

    let mut message = MessageBuilder::new();

    let (action, user, preposition, role) = match action.as_str() {
        "member_invited" => {
            let invitation = event
                .invitation
                .expect("member was invited but no invitation is set");
            let user = event.user.expect("member was invited but no user is set");

            ("invited", user, "to", invitation.role)
        }
        "member_added" => {
            let membership = event
                .membership
                .expect("member was added but no membership is set");
            let user = membership.user;

            ("added", user, "to", membership.role)
        }
        "member_removed" => {
            let membership = event
                .membership
                .expect("member was removed but no membership is set");
            let user = membership.user;

            ("removed", user, "from", membership.role)
        }

        // TODO maybe handle `renamed` and `deleted` actions even tho it should not happen in our case
        _ => {
            error!("invalid or unsupported organization action: {}", action);
            return None;
        }
    };

    write!(&mut message, "{} {} ", event.sender.login, action).unwrap();
    message.link(&user.login, &user.html_url);
    write!(&mut message, " {} organization", preposition).unwrap();

    match action {
        "invited" | "added" => write!(&mut message, " as {}", role).unwrap(),
        "removed" => write!(&mut message, " (was {})", role).unwrap(),
        _ => unreachable!(),
    };

    Some(Response {
        message,
        repo: None,
    })
}

fn handle_ping(event: PingEvent) -> Option<Response> {
    let mut message = MessageBuilder::new();

    match &(event.repository) {
        Some(repo) => {
            message.tag(&repo.name, Some(emoji::PING_PONG));
            write!(&mut message, " ").unwrap();
        }
        None => {}
    }

    write!(
        &mut message,
        "{} completed webhook setup! {}",
        event.sender.login, event.zen
    )
    .unwrap();

    Some(Response {
        message,
        repo: event.repository.map(|r| r.full_name),
    })
}

fn handle_pull_request(event: PullRequestEvent) -> Option<Response> {
    let action = event.action;
    let pr = event.pull_request;

    let mut message = MessageBuilder::new();

    message.tag(&event.repository.name, Some(emoji::OUTBOX_TRAY));

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
            message.main_link(&format!("{}", pr), &pr.html_url);
        }

        "review_requested" => {
            let reviewers = pr
                .requested_reviewers
                .iter()
                .map(|user| user.login.as_str())
                .collect::<Vec<&str>>()
                .join(", ");

            write!(message, " requested {} to review ", reviewers).unwrap();
            message.main_link(&format!("{}", pr), &pr.html_url);
        }

        // too verbose, don't log that
        "labeled" | "unlabeled" | "review_requested_removed" => return None,

        "opened" | "edited" | "reopened" => {
            let base = &pr.base.r#ref;
            let head = &pr.head.r#ref;
            write!(message, " {} ", action).unwrap();
            message.main_link(&format!("{}", pr), &pr.html_url);
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
            message.main_link(&format!("{}", pr), &pr.html_url);
        }

        _ => {
            error!("invalid or unsupported pull request action: {}", action);
            return None;
        }
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
        "commented" => "commented on",
        _ => {
            error!(
                "invalid or unsupported pull request review state: {}",
                state
            );
            return None;
        }
    };

    let mut message = MessageBuilder::new();

    message.tag(&event.repository.name, Some(emoji::OUTBOX_TRAY));
    write!(&mut message, " {}", event.sender.login).unwrap();

    match action.as_str() {
        "submitted" => {
            write!(message, " {} ", decision).unwrap();
            message.main_link(&format!("{}", pr), &pr.html_url);
        }

        // ignored, too verbose
        "edited" => return None,

        "dismissed" => {
            write!(message, " dismissed ").unwrap();

            let mut whose = String::new();
            if event.sender.login == reviewer {
                write!(whose, "their").unwrap();
            } else {
                write!(whose, "{}'s", reviewer).unwrap();
            };

            message.main_link(&format!("{} review", whose), &review.html_url);

            write!(message, " for ").unwrap();
            message.link(&format!("{}", pr), &pr.html_url);
            write!(message, " (they {} the PR)", decision).unwrap();
        }

        _ => {
            error!(
                "invalid or unsupported pull request review action: {}",
                action
            );
            return None;
        }
    }

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

    message.tag(&event.repository.name, Some(emoji::SPEECH_BALLOON));

    write!(&mut message, " {} ", event.sender.login).unwrap();

    match action.as_str() {
        "created" => {
            message.main_link("commented", &comment.html_url);
            write!(message, " on ").unwrap();
            message.link(&format!("{}", pr), &pr.html_url);

            // comment can be on a specific line of a file
            if let Some(location) = comment.location() {
                write!(message, " {}", location,).unwrap();
            }
        }

        // ignored, too verbose
        "edited" | "deleted" => return None,

        _ => {
            error!(
                "invalid or unsupported pull request review comment action: {}",
                action
            );
            return None;
        }
    }

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

    message.tag(&event.repository.name, None);

    write!(&mut message, " {} {}pushed ", pusher, force).unwrap();

    let url: &Url;
    let mut text = String::new();

    if commits.len() == 1 {
        write!(text, "{}", hash).unwrap();
        url = &head.url;
    } else {
        write!(text, "{} commits", commits.len()).unwrap();

        let distinct_count = commits.iter().filter(|c| c.distinct).count();
        if distinct_count != commits.len() {
            write!(text, " ({} distinct)", distinct_count).unwrap();
        }

        write!(text, " including {}", hash).unwrap();

        url = &event.compare;
    }
    message.main_link(&text, url);

    let branch = event
        .r#ref
        .strip_prefix("refs/heads/")
        .expect("couldn't find branch name");

    write!(message, " on ").unwrap();
    if event.created {
        write!(message, "new ").unwrap();
    }

    let ref_url = match event.repository.ref_url(branch) {
        Ok(url) => url,
        Err(e) => {
            error!(
                "couldn't build ref url for branch {} in repo {}: {}",
                branch, event.repository.full_name, e
            );
            event.repository.html_url
        }
    };

    message.link(&format!("{}{}", BRANCH, branch), &ref_url);
    write!(message, ": {}", shorten_content(head.title())).unwrap();

    Some(Response {
        message,
        repo: Some(event.repository.full_name),
    })
}

fn handle_repository(event: RepositoryEvent) -> Option<Response> {
    let mut message = MessageBuilder::new();

    match event.action.as_str() {
        "created" | "deleted" | "archived" | "unarchived" | "transferred" | "publicized"
        | "privatized" => {
            message.tag(&event.repository.name, Some(emoji::PACKAGE));

            write!(
                &mut message,
                " {} {} repository",
                event.sender.login, event.action
            )
            .unwrap();
        }

        "renamed" => {
            let old_repo_name = event
                .changes
                .expect("no changes reported even if repository was renamed")
                .repository
                .name
                .from;

            message.tag(&old_repo_name, Some(emoji::PACKAGE));

            write!(
                &mut message,
                " {} renamed repository to {}",
                event.sender.login, event.repository.name
            )
            .unwrap();
        }

        "edited" => return None, // ignore, too verbose

        _ => {
            error!("invalid or unsupported repository action: {}", event.action);
            return None;
        }
    }

    Some(Response {
        message,
        repo: Some(event.repository.full_name),
    })
}

#[cfg(test)]
mod tests {
    use crate::webhooks::github::{
        Comment, Commit, CommitCommentEvent, ForkEvent, GitHubUser, Issue, MembershipEvent,
        OrganizationMembership, PrRef, PullRequest, Repository, Review, Team,
    };

    use super::*;

    #[test]
    fn test_handle_commit_comment() {
        let event = CommitCommentEvent {
            sender: GitHubUser {
                login: "test-user".to_string(),
                id: 42,
                html_url: Url::parse("https://github.com/test-user").unwrap(),
            },
            repository: Repository {
                name: "test-repo".to_string(),
                full_name: "test-user/test-repo".to_string(),
                html_url: Url::parse("https://github.com/test-user/test-repo").unwrap(),
            },
            comment: Comment {
                html_url: Url::parse("https://github.com/test-user/test-repo/issues/42#issue-42424242").unwrap(),
                body: "This content is very long, longer than our character limit, so it will definitely be truncated".to_string(),
                commit_id: Some("4242424242424242424242424242424242424242".to_string()),
                pull_request_review_id: None,
                path: None,
                position: None,
            },
        };

        let response = handle_commit_comment(event).expect("should have a response");

        let message = response.message;

        assert!(message.url.is_some());

        assert_eq!(message.plain, "[💬 test-repo] test-user commented on 4242424: This content is very long, longer than our character limit, so it will d…",);

        assert_eq!(
            message.html,
            r#"<b>[💬 test-repo]</b> test-user <a href="https://github.com/test-user/test-repo/issues/42#issue-42424242">commented</a> on <a href="https://github.com/test-user/test-repo/issues/42">4242424</a>: This content is very long, longer than our character limit, so it will d…"#,
        );
    }

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
                html_url: Url::parse("https://github.com/test-user").unwrap(),
            },
            r#ref: "test-tag".to_string(),
        };

        let response = handle_create(event).expect("should have a response");

        let message = response.message;

        assert!(message.url.is_some());

        assert_eq!(message.plain, "[test-repo] test-user created tag test-tag",);

        assert_eq!(
            message.html,
            r#"<b>[test-repo]</b> test-user created tag <a href="https://github.com/test-user/test-repo/tree/test-tag">test-tag</a>"#,
        );
    }

    #[test]
    fn test_handle_fork() {
        let event = ForkEvent {
            forkee: Repository {
                name: "test-repo".to_string(),
                full_name: "test-user2/test-repo".to_string(),
                html_url: Url::parse("https://github.com/test-user2/test-repo").unwrap(),
            },
            repository: Repository {
                name: "test-repo".to_string(),
                full_name: "test-user/test-repo".to_string(),
                html_url: Url::parse("https://github.com/test-user/test-repo").unwrap(),
            },
            sender: GitHubUser {
                login: "test-user2".to_string(),
                id: 420,
                html_url: Url::parse("https://github.com/test-user").unwrap(),
            },
        };

        let response = handle_fork(event).expect("should have a response");

        let message = response.message;

        assert!(message.url.is_some());

        assert_eq!(
            message.plain,
            "[📦 test-repo] test-user2 forked into test-user2/test-repo",
        );

        assert_eq!(
            message.html,
            r#"<b>[📦 test-repo]</b> <a href="https://github.com/test-user">test-user2</a> forked into <a href="https://github.com/test-user2/test-repo">test-user2/test-repo</a>"#,
        );
    }

    #[test]
    fn test_handle_issue_comment() {
        let event = IssueCommentEvent {
            sender: GitHubUser {
                login: "test-user".to_string(),
                id: 42,
                html_url: Url::parse("https://github.com/test-user").unwrap(),
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
                commit_id: None,
                pull_request_review_id: None,
                path: None,
                position: None,
            },
        };

        let response = handle_issue_comment(event).expect("should have a response");

        let message = response.message;

        assert!(message.url.is_some());

        assert_eq!(
            message.plain,
            "[🔧 test-repo] test-user commented on issue #42 (Test Issue Title): This content is very long, longer than our character limit, so it will d…",
        );

        assert_eq!(
            message.html,
            r#"<b>[🔧 test-repo]</b> test-user <a href="https://github.com/test-user/test-repo/issues/42#issue-42424242">commented</a> on issue <a href="https://github.com/test-user/test-repo/issues/42">#42 (Test Issue Title)</a>: This content is very long, longer than our character limit, so it will d…"#,
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
                html_url: Url::parse("https://github.com/test-user").unwrap(),
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

        assert!(message.url.is_some());

        assert_eq!(
            message.plain,
            "[🔧 test-repo] test-user opened issue #42 (Test Issue Title)",
        );

        assert_eq!(
            message.html,
            r#"<b>[🔧 test-repo]</b> test-user opened issue <a href="https://github.com/test-user/test-repo/issues/42">#42 (Test Issue Title)</a>"#,
        );
    }

    #[test]
    fn test_handle_membership() {
        let event = MembershipEvent {
            action: "added".to_string(),
            member: GitHubUser {
                login: "test-user".to_string(),
                id: 42,
                html_url: Url::parse("https://github.com/test-user").unwrap(),
            },
            team: Team {
                name: "test-team".to_string(),
                id: 42,
                description: String::new(),
                privacy: "closed".to_string(),
                permission: "pull".to_string(),
                html_url: Url::parse("https://github.com/orgs/test-org/teams/test-team").unwrap(),
            },
            sender: GitHubUser {
                login: "test-admin".to_string(),
                id: 4242,
                html_url: Url::parse("https://github.com/test-user").unwrap(),
            },
        };

        let response = handle_membership(event).expect("should have a response");

        let message = response.message;

        assert!(message.url.is_none());

        assert_eq!(
            message.plain,
            "[🧑 test-team] test-admin added test-user to the team",
        );

        assert_eq!(
            message.html,
            r#"<b>[🧑 test-team]</b> test-admin added <a href="https://github.com/test-user">test-user</a> to the team"#,
        );
    }

    #[test]
    fn test_handle_organization() {
        let event = OrganizationEvent {
            action: "member_added".to_string(),
            sender: GitHubUser {
                login: "test-admin".to_string(),
                id: 4242,
                html_url: Url::parse("https://github.com/test-user").unwrap(),
            },
            invitation: None,
            user: None,
            membership: Some(OrganizationMembership {
                role: "member".to_string(),
                user: GitHubUser {
                    login: "test-user".to_string(),
                    id: 42,
                    html_url: Url::parse("https://github.com/test-user").unwrap(),
                },
            }),
        };

        let response = handle_organization(event).expect("should have a response");

        let message = response.message;

        assert!(message.url.is_none());

        assert_eq!(
            message.plain,
            "test-admin added test-user to organization as member",
        );

        assert_eq!(
            message.html,
            r#"test-admin added <a href="https://github.com/test-user">test-user</a> to organization as member"#,
        );
    }

    #[test]
    fn test_handle_ping() {
        let event = PingEvent {
            zen: "Follow the rules, you must!".to_string(),
            repository: Some(Repository {
                name: "test-repo".to_string(),
                full_name: "test-user/test-repo".to_string(),
                html_url: Url::parse("https://github.com/test-user/test-repo").unwrap(),
            }),
            sender: GitHubUser {
                login: "test-user".to_string(),
                id: 42,
                html_url: Url::parse("https://github.com/test-user").unwrap(),
            },
        };

        let response = handle_ping(event).expect("should have a response");

        let message = response.message;

        assert!(message.url.is_none());

        assert_eq!(
            message.plain,
            "[🏓 test-repo] test-user completed webhook setup! Follow the rules, you must!",
        );

        assert_eq!(
            message.html,
            r#"<b>[🏓 test-repo]</b> test-user completed webhook setup! Follow the rules, you must!"#,
        );
    }

    #[test]
    fn test_handle_pull_request() {
        let event = PullRequestEvent {
            sender: GitHubUser {
                login: "test-user".to_string(),
                id: 42,
                html_url: Url::parse("https://github.com/test-user").unwrap(),
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
                    html_url: Url::parse("https://github.com/test-user").unwrap(),
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

        assert!(message.url.is_some());

        assert_eq!(
            message.plain,
            "[📤 test-repo] test-user opened PR #42: Test PR Title by test-user (main...test)",
        );

        assert_eq!(
            message.html,
            r#"<b>[📤 test-repo]</b> test-user opened <a href="https://github.com/test-user/test-repo/pull/42">PR #42: Test PR Title by test-user</a> (main...test)"#,
        );
    }

    #[test]
    fn test_handle_pull_request_review() {
        let event = PullRequestReviewEvent {
            sender: GitHubUser {
                login: "test-user".to_string(),
                id: 42,
                html_url: Url::parse("https://github.com/test-user").unwrap(),
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
                    html_url: Url::parse("https://github.com/test-user").unwrap(),
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
            action: "dismissed".to_string(),
            review: Review {
                state: "approved".to_string(),
                user: GitHubUser {
                    login: "test-user".to_string(),
                    id: 42,
                    html_url: Url::parse("https://github.com/test-user").unwrap(),
                },
                html_url: Url::parse("https://github.com/test-user/test-repo/whatever").unwrap(),
            },
        };

        let response = handle_pull_request_review(event).expect("should have a response");

        let message = response.message;

        assert!(message.url.is_some());

        assert_eq!(
            message.plain,
            "[📤 test-repo] test-user dismissed their review for PR #42: Test PR Title by test-user (they approved the PR)"
        );

        assert_eq!(
            message.html,
            r#"<b>[📤 test-repo]</b> test-user dismissed <a href="https://github.com/test-user/test-repo/whatever">their review</a> for <a href="https://github.com/test-user/test-repo/pull/42">PR #42: Test PR Title by test-user</a> (they approved the PR)"#,
        );
    }

    #[test]
    fn test_handle_pull_request_review_comment() {
        let event = PullRequestReviewCommentEvent {
            sender: GitHubUser {
                login: "test-user".to_string(),
                id: 42,
                html_url: Url::parse("https://github.com/test-user").unwrap(),
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
                html_url: Url::parse("https://github.com/test-user").unwrap(),
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
            action: "created".to_string(),
            comment: Comment {
                html_url: Url::parse("https://github.com/test-user/test-repo/whatever").unwrap(),
                body: "This content is very long, longer than our character limit, so it will definitely be truncated".to_string(),
                commit_id: None,
                pull_request_review_id: None,
                path: None,
                position: None,
            },
        };

        let response = handle_pull_request_review_comment(event).expect("should have a response");

        let message = response.message;

        assert!(message.url.is_some());

        assert_eq!(
            message.plain,
            "[💬 test-repo] test-user commented on PR #42: Test PR Title by test-user"
        );

        assert_eq!(
            message.html,
            r#"<b>[💬 test-repo]</b> test-user <a href="https://github.com/test-user/test-repo/whatever">commented</a> on <a href="https://github.com/test-user/test-repo/pull/42">PR #42: Test PR Title by test-user</a>"#,
        );
    }

    #[test]
    fn test_handle_push() {
        let event = PushEvent {
            repository: Repository {
                name: "test-repo".to_string(),
                full_name: "test-user/test-repo".to_string(),
                html_url: Url::parse("https://github.com/test-user/test-repo").unwrap(),
            },
            sender: GitHubUser {
                login: "test-user".to_string(),
                id: 42,
                html_url: Url::parse("https://github.com/test-user").unwrap(),
            },
            commits: vec![
                Commit {
                    id: "deadbeef".to_string(),
                    url: Url::parse("https://github.com/test-user/test-repo/commit/deadbeef").unwrap(),
                    distinct: true,
                    message: "This content is very long, longer than our character limit, so it will definitely be truncated".to_string(),
                },

                Commit {
                    id: "beefdead".to_string(),
                    url: Url::parse("https://github.com/test-user/test-repo/commit/beefdead").unwrap(),
                    distinct: true,
                    message: "Another message".to_string(),
                }

            ],
            head_commit: Some(Commit {
                id: "deadbeef".to_string(),
                url: Url::parse("https://github.com/test-user/test-repo/commit/deadbeef").unwrap(),
                distinct: true,
                message: "This content is very long, longer than our character limit, so it will definitely be truncated".to_string(),
            }),
            forced: true,
            created: true,
            compare: Url::parse(
                "https://github.com/test-user/test-repo/compare/deadbeef...beefdead",
            )
                .unwrap(),
            r#ref: "refs/heads/new-test-branch".to_string(),
        };

        let response = handle_push(event).expect("should have a response");

        let message = response.message;

        assert!(message.url.is_some());

        assert_eq!(
            message.plain,
            "[test-repo] test-user force-pushed 2 commits including deadbee on new ⊶new-test-branch: This content is very long, longer than our character limit, so it will d…",
        );

        assert_eq!(
            message.html,
            r#"<b>[test-repo]</b> test-user force-pushed <a href="https://github.com/test-user/test-repo/compare/deadbeef...beefdead">2 commits including deadbee</a> on new <a href="https://github.com/test-user/test-repo/tree/new-test-branch">⊶new-test-branch</a>: This content is very long, longer than our character limit, so it will d…"#,
        );
    }

    #[test]
    fn test_handle_repository() {
        let event = RepositoryEvent {
            action: "created".to_string(),
            repository: Repository {
                name: "test-repo".to_string(),
                full_name: "test-user/test-repo".to_string(),
                html_url: Url::parse("https://github.com/test-user/test-repo").unwrap(),
            },
            sender: GitHubUser {
                login: "test-user".to_string(),
                id: 42,
                html_url: Url::parse("https://github.com/test-user").unwrap(),
            },
            changes: None,
        };

        let response = handle_repository(event).expect("should have a response");

        let message = response.message;

        assert!(message.url.is_none());

        assert_eq!(message.plain, "[📦 test-repo] test-user created repository",);

        assert_eq!(
            message.html,
            r#"<b>[📦 test-repo]</b> test-user created repository"#,
        );
    }
}
