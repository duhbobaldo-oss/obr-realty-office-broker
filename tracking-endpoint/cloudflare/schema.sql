-- OBR Track — Cloudflare D1 schema (PROTOTYPE — requires confirmation against
-- the existing production database before any manual deployment).
-- Apply ONLY after verifying the existing tracked_links / click_events schema
-- (see report: UNKNOWN / REQUIRES CONFIRMATION).

CREATE TABLE IF NOT EXISTS tracked_links (
  tracking_code TEXT PRIMARY KEY,
  original_ad_url TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1,
  owner_installation TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS click_events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  tracking_code TEXT NOT NULL,
  visitor_hash TEXT NOT NULL,
  counted_raw INTEGER NOT NULL DEFAULT 1,
  is_bot INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  FOREIGN KEY (tracking_code) REFERENCES tracked_links(tracking_code)
);

CREATE INDEX IF NOT EXISTS idx_clicks_code_time ON click_events(tracking_code, created_at);
CREATE INDEX IF NOT EXISTS idx_clicks_code_visitor ON click_events(tracking_code, visitor_hash);
