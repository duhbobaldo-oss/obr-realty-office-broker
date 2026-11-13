import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import Database from 'better-sqlite3';

const DEFAULT_DB = path.join(process.cwd(), 'data', 'tracking.sqlite');

export function openDb(dbPath = process.env.OBR_TRACKING_DB || DEFAULT_DB) {
  fs.mkdirSync(path.dirname(dbPath), { recursive: true });
  const db = new Database(dbPath);
  db.pragma('journal_mode = WAL');
  db.exec(`
    CREATE TABLE IF NOT EXISTS tracked_links (
      tracking_code TEXT PRIMARY KEY,
      original_ad_url TEXT NOT NULL,
      enabled INTEGER NOT NULL DEFAULT 1,
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
  `);
  // ترقية سلمية: مالك الرابط (installation_id للتثبيت المنشئ) — لروابط v2
  try {
    db.exec('ALTER TABLE tracked_links ADD COLUMN owner_installation TEXT');
  } catch {
    // العمود موجود مسبقًا
  }
  return db;
}

export function upsertLink(db, { tracking_code, original_ad_url, enabled = true, owner_installation = null }) {
  db.prepare(`
    INSERT INTO tracked_links (tracking_code, original_ad_url, enabled, owner_installation, created_at, updated_at)
    VALUES (@tracking_code, @original_ad_url, @enabled, @owner_installation, datetime('now'), datetime('now'))
    ON CONFLICT(tracking_code) DO UPDATE SET
      original_ad_url = excluded.original_ad_url,
      enabled = excluded.enabled,
      updated_at = datetime('now')
  `).run({
    tracking_code,
    original_ad_url,
    enabled: enabled ? 1 : 0,
    owner_installation
  });
}

export function patchLink(db, tracking_code, patch) {
  const row = db.prepare('SELECT * FROM tracked_links WHERE tracking_code = ?').get(tracking_code);
  if (!row) return null;
  const original_ad_url = patch.original_ad_url ?? row.original_ad_url;
  const enabled = patch.enabled == null ? row.enabled : (patch.enabled ? 1 : 0);
  db.prepare(`
    UPDATE tracked_links
    SET original_ad_url = ?, enabled = ?, updated_at = datetime('now')
    WHERE tracking_code = ?
  `).run(original_ad_url, enabled, tracking_code);
  return db.prepare('SELECT * FROM tracked_links WHERE tracking_code = ?').get(tracking_code);
}

export function getLink(db, tracking_code) {
  return db.prepare('SELECT * FROM tracked_links WHERE tracking_code = ?').get(tracking_code) || null;
}

export function countLinksByOwner(db, owner_installation) {
  return Number(db.prepare(
    'SELECT COUNT(*) AS n FROM tracked_links WHERE owner_installation = ?'
  ).get(owner_installation).n) || 0;
}

export function visitorHash(ip, ua) {
  const raw = `${ip || '0.0.0.0'}|${(ua || '').slice(0, 180)}`;
  return crypto.createHash('sha256').update(raw).digest('hex').slice(0, 32);
}

export function getStats(db, tracking_code) {
  const link = getLink(db, tracking_code);
  if (!link) return null;
  const tracked = db.prepare(`
    SELECT COUNT(*) AS n FROM click_events
    WHERE tracking_code = ? AND is_bot = 0 AND counted_raw = 1
  `).get(tracking_code).n;
  const unique = db.prepare(`
    SELECT COUNT(DISTINCT visitor_hash) AS n FROM click_events
    WHERE tracking_code = ? AND is_bot = 0 AND counted_raw = 1
  `).get(tracking_code).n;
  const last = db.prepare(`
    SELECT created_at FROM click_events
    WHERE tracking_code = ? AND is_bot = 0 AND counted_raw = 1
    ORDER BY id DESC LIMIT 1
  `).get(tracking_code);
  return {
    tracked_clicks: Number(tracked) || 0,
    unique_clicks: Number(unique) || 0,
    last_click_at: last?.created_at || null,
    enabled: !!link.enabled
  };
}

/** Dedup window seconds for rapid reload (same visitor). */
export const RAW_DEDUP_SECONDS = 45;

export function recordClick(db, { tracking_code, visitor_hash, is_bot }) {
  if (is_bot) {
    db.prepare(`
      INSERT INTO click_events (tracking_code, visitor_hash, counted_raw, is_bot, created_at)
      VALUES (?, ?, 0, 1, datetime('now'))
    `).run(tracking_code, visitor_hash);
    return { counted: false, reason: 'bot' };
  }

  const recent = db.prepare(`
    SELECT id FROM click_events
    WHERE tracking_code = ?
      AND visitor_hash = ?
      AND is_bot = 0
      AND counted_raw = 1
      AND created_at >= datetime('now', ?)
    LIMIT 1
  `).get(tracking_code, visitor_hash, `-${RAW_DEDUP_SECONDS} seconds`);

  if (recent) {
    db.prepare(`
      INSERT INTO click_events (tracking_code, visitor_hash, counted_raw, is_bot, created_at)
      VALUES (?, ?, 0, 0, datetime('now'))
    `).run(tracking_code, visitor_hash);
    return { counted: false, reason: 'dedup' };
  }

  db.prepare(`
    INSERT INTO click_events (tracking_code, visitor_hash, counted_raw, is_bot, created_at)
    VALUES (?, ?, 1, 0, datetime('now'))
  `).run(tracking_code, visitor_hash);
  return { counted: true, reason: 'ok' };
}
