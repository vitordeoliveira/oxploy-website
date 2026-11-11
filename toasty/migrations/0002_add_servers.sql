CREATE TABLE "server_days" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "date" TEXT NOT NULL,
    "server_id" INTEGER NOT NULL
);
-- #[toasty::breakpoint]
CREATE INDEX "index_server_days_by_date" ON "server_days" ("date");
-- #[toasty::breakpoint]
CREATE INDEX "index_server_days_by_server_id" ON "server_days" ("server_id");
-- #[toasty::breakpoint]
CREATE TABLE "servers" (
    "id" INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    "key" TEXT NOT NULL,
    "first_seen" TEXT NOT NULL,
    "last_seen" TEXT NOT NULL,
    "version" TEXT NOT NULL,
    "platform" TEXT NOT NULL,
    "usage" TEXT
);
-- #[toasty::breakpoint]
CREATE UNIQUE INDEX "index_servers_by_key" ON "servers" ("key");
