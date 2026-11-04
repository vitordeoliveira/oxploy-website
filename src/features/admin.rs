//! The site's admin: one person, whose name and password are in the environment, as
//! `ADMIN_USERNAME` and `ADMIN_PASSWORD`. Without them, nobody can sign in.
//!
//! Topcoat gives the signed-in browser a random token in a cookie. Only its hash is
//! stored here.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};
use topcoat::{Result, context::Cx, session};

use crate::db::db;

/// A browser signed in as the admin.
#[derive(Debug, toasty::Model)]
pub struct AdminSession {
    #[key]
    #[auto]
    pub id: u64,

    /// The SHA-256 hash of the session token.
    #[unique]
    pub token_hash: Vec<u8>,

    /// When the session stops being valid, in seconds since the Unix epoch.
    pub expires_at: i64,
}

/// How long a wrong name or password makes the browser wait, which slows guessing.
const WRONG_WAIT: Duration = Duration::from_secs(1);

/// The admin's name and password, if the environment has both.
fn credentials() -> Option<(String, String)> {
    let name = std::env::var("ADMIN_USERNAME").ok().filter(|name| !name.is_empty())?;
    let password = std::env::var("ADMIN_PASSWORD").ok().filter(|password| !password.is_empty())?;
    Some((name, password))
}

/// Whether the environment sets the admin's name and password.
pub fn is_set_up() -> bool {
    credentials().is_some()
}

/// Signs the browser in if `name` and `password` are the admin's. Returns whether they
/// were.
pub async fn sign_in(cx: &Cx, name: &str, password: &str) -> Result<bool> {
    let matches = credentials().is_some_and(|(admin, secret)| {
        same(&admin, name) & same(&secret, password)
    });
    if !matches {
        tokio::time::sleep(WRONG_WAIT).await;
        return Ok(false);
    }
    let session = session::start(cx).await?;
    toasty::create!(AdminSession {
        token_hash: session.token_hash.to_vec(),
        expires_at: unix_seconds(session.expires_at),
    })
    .exec(&mut db(cx))
    .await?;
    Ok(true)
}

/// Signs the browser out and forgets its session.
pub async fn sign_out(cx: &Cx) -> Result<()> {
    if let Some(hash) = session::stop(cx).await? {
        AdminSession::filter_by_token_hash(hash.to_vec())
            .delete()
            .exec(&mut db(cx))
            .await?;
    }
    Ok(())
}

/// Whether the browser is signed in as the admin, which takes a session that has not
/// expired, and the admin's name and password still in the environment.
pub async fn signed_in(cx: &Cx) -> Result<bool> {
    if !is_set_up() {
        return Ok(false);
    }
    let Some(hash) = session::token_hash(cx).await? else {
        return Ok(false);
    };
    let mut db = db(cx);
    let Some(stored) = AdminSession::filter_by_token_hash(hash.to_vec())
        .first()
        .exec(&mut db)
        .await?
    else {
        return Ok(false);
    };
    if stored.expires_at <= unix_seconds(SystemTime::now()) {
        AdminSession::filter_by_id(stored.id).delete().exec(&mut db).await?;
        return Ok(false);
    }
    Ok(true)
}

/// Whether two texts are the same, comparing their hashes in full, so the time taken
/// tells nothing of how much of a guess was right.
fn same(expected: &str, given: &str) -> bool {
    let (expected, given) = (Sha256::digest(expected), Sha256::digest(given));
    expected
        .iter()
        .zip(given.iter())
        .fold(0, |differs, (a, b)| differs | (a ^ b))
        == 0
}

fn unix_seconds(time: SystemTime) -> i64 {
    let seconds = time.duration_since(UNIX_EPOCH).map_or(0, |since| since.as_secs());
    i64::try_from(seconds).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn texts_compare_in_full() {
        assert!(same("correct horse", "correct horse"));
        assert!(!same("correct horse", "correct hors"));
        assert!(!same("correct horse", ""));
    }
}
