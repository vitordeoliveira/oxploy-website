CREATE TABLE "github_days" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "date" TEXT NOT NULL,
    "stars" BIGINT NOT NULL,
    "downloads" BIGINT
);
-- #[toasty::breakpoint]
CREATE UNIQUE INDEX "index_github_days_by_date" ON "github_days" ("date");
-- #[toasty::breakpoint]
CREATE TABLE "install_fetches" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "at" BIGINT NOT NULL,
    "machine" TEXT NOT NULL
);
-- #[toasty::breakpoint]
CREATE INDEX "index_install_fetches_by_at" ON "install_fetches" ("at");
