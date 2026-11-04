CREATE TABLE "analytics_keys" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "key" BLOB NOT NULL
);
-- #[toasty::breakpoint]
CREATE TABLE "admin_sessions" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "token_hash" BLOB NOT NULL,
    "expires_at" BIGINT NOT NULL
);
-- #[toasty::breakpoint]
CREATE UNIQUE INDEX "index_admin_sessions_by_token_hash" ON "admin_sessions" ("token_hash");
-- #[toasty::breakpoint]
CREATE TABLE "page_views" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "at" BIGINT NOT NULL,
    "path" TEXT NOT NULL,
    "visitor" TEXT NOT NULL,
    "referrer" TEXT,
    "device" TEXT,
    "browser" TEXT,
    "campaign" TEXT,
    "browser_id" TEXT
);
-- #[toasty::breakpoint]
CREATE INDEX "index_page_views_by_at" ON "page_views" ("at");
