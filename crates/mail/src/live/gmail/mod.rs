//! Gmail over the Gmail REST API, opened with access tokens from Superhuman. Labels become
//! folders, threads become list rows, and messages are mapped onto the same
//! records the JMAP backend produces, so the rest of the app cannot tell the
//! two apart.

mod api;
mod convert;
mod mime;
mod search;
mod snooze;
mod workspace;

pub(crate) use workspace::GmailWorkspace;
