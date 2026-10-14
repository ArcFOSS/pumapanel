-- Minecraft servers managed by PumaPanel.
--
-- Only metadata lives here. The container itself belongs to the PumaPanel
-- agent, which derives its state from container labels; this row is the panel's
-- record of the server. `id` is the join key between the two: it names both the
-- container and its data volume (`pumapanel-<id>`).
--
-- `host_port` is UNIQUE so two servers can never be published on the same port;
-- the application also checks for conflicts to return a friendly error, but the
-- constraint is what makes it true under concurrency.
CREATE TABLE IF NOT EXISTS servers (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    version TEXT NOT NULL,
    memory_mb INTEGER NOT NULL,
    host_port INTEGER NOT NULL UNIQUE,
    -- Minimum Java reported by PaperMC, used to pick the image tag. NULL when
    -- the version catalogue was unavailable at creation time.
    java_min INTEGER,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
