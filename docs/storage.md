Each release of an app runs from a fresh copy of its code, so anything the app writes
there goes with the next deploy. What the app keeps belongs in its storage folder.

## The storage folder

oxploy gives each service a folder, and passes its path to the app and to the script
after the build in `OXPLOY_STORAGE_DIR`. Deploys and restarts keep it; deleting the
service deletes it.

```rust
let storage = std::env::var("OXPLOY_STORAGE_DIR")?;
let uploads = std::path::Path::new(&storage).join("uploads");
```

The **Storage** tab shows what is in it.

![The Storage tab, with the storage folder, a managed database and its backup](storage)

## SQLite databases

The **Storage** tab also creates SQLite databases in the storage folder and connects them
to the app, in one of two ways or both:

- **Variable**: oxploy sets a variable, such as `DATABASE_URL`, to the database, for the
  app, its builds and the script after the build. Choose the value's form:
  `sqlite:/…/data.db` for toasty and sqlx, or `/…/data.db` for rusqlite and diesel.
- **Link**: for apps with the path in their code, oxploy puts a link to the database in
  each release, at a path from the build path, such as `data.db`.

oxploy connects a new database the next time the app starts. It can also take over a
database file the app already has in its storage folder: **Manage** beside it.

If the app has migrations, run them in the [script after the
build](/docs/deploying#run-a-script-after-the-build), which runs with the databases
already linked.

## Backups

**Back up** copies a database, and the copy is ready to download. Restoring a backup stops
the app, backs up the current database, puts the copy in its place and starts the app
again. Deleting a database backs it up first too.

Backups live beside the storage folder, not in it, so a restore cannot overwrite them.
