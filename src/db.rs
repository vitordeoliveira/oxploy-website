//! The database: the site's visits and the admin's sessions.

use toasty::{Db, migration::MigrationSet};
use topcoat::context::{Cx, app_context};

/// The migrations in `toasty/`, built into the binary.
static MIGRATIONS: MigrationSet = toasty::embed_migrations!();

/// Opens the database with every model of the site and applies any pending migrations,
/// so a fresh install creates its own tables.
///
/// After changing a model, generate a migration with
/// `cargo run -- toasty migration generate --name <what_changed>`.
pub async fn connect() -> toasty::Result<Db> {
    let db = Db::builder()
        .models(toasty::models!(crate::*))
        .connect(&url())
        .await?;
    MIGRATIONS.apply(&db).await?;
    Ok(db)
}

/// `DATABASE_URL`, or else a SQLite file in the storage folder oxploy gives the site,
/// which its deploys keep, or else `data.db` where the site runs.
fn url() -> String {
    if let Ok(url) = std::env::var("DATABASE_URL") {
        return url;
    }
    match std::env::var("OXPLOY_STORAGE_DIR") {
        Ok(storage) => format!("sqlite:{storage}/website.db"),
        Err(_) => "sqlite:data.db".to_owned(),
    }
}

/// Returns the database handle from the router's app context. Cloning it is cheap
/// because it refers to a shared connection pool.
pub fn db(cx: &Cx) -> Db {
    app_context::<Db>(cx).clone()
}
