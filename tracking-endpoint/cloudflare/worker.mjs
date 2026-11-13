/**
 * OBR Track — Cloudflare Worker (v2 keyless protocol) — PROTOTYPE FOR REVIEW.
 *
 * Prototype based on local tracking-endpoint v2 logic. NOT deployed.
 * Requires manual confirmation of: D1 binding name, existing production
 * schema/data compatibility, and current /t/:code behavior (see report).
 *
 * Bindings required (wrangler.toml):
 *   [[d1_databases]]
 *   binding = "DB"            # <- confirm actual production binding name
 *   database_name = "obr-track"
 *   database_id  = "<from dashboard>"
 *
 * Endpoints:
 *   GET   /health
 *   POST  /v2/links            { destination_url, installation_id, product_id }
 *   PATCH /v2/links/:code      { original_ad_url?, enabled? }   (owner only)
 *   GET   /v2/links/:code/stats                                 (owner only)
 *   GET   /t/:code             public 302 redirect + click record
 */

const PRODUCT_RE = /^[a-z0-9-]{1,64}$/;
const INSTALLATION_RE = /^[A-Za-z0-9_-]{1,64}$/;
const CODE_RE = /^[a-f0-9]{48}$/;
const MAX_LINKS_PER_INSTALLATION = 5000;
const DEDUP_SECONDS = 45;

function json(data, status = 200, extraHeaders = {}) {
  return new Response(JSON.stringify(data), {
    status,
    headers: { "content-type": "application/json; charset=utf-8", ...extraHeaders },
  });
}

function clientIp(req) {
  return req.headers.get("cf-connecting-ip") || "0.0.0.0";
}

async function sha256hex(s) {
  const buf = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(s));
  return [...new Uint8Array(buf)].map((b) => b.toString(16).padStart(2, "0")).join("");
}

async function visitorHashOf(req) {
  const ua = (req.headers.get("user-agent") || "").slice(0, 180);
  return sha256hex(`${clientIp(req)}|${ua}`).then((h) => h.slice(0, 32));
}

/** In-memory rate limiter (per-isolate). Good enough for v1 of the Worker;
 *  cross-isolate enforcement would need Durable Objects — see report. */
const buckets = new Map();
function rateLimit(key, max, windowMs) {
  const now = Date.now();
  let arr = buckets.get(key);
  if (!arr) { arr = []; buckets.set(key, arr); }
  while (arr.length && now - arr[0] > windowMs) arr.shift();
  if (arr.length >= max) return false;
  arr.push(now);
  return true;
}

function v2Owns(link, installation) {
  if (!link.owner_installation) return true; // legacy v1 rows: no owner recorded
  return link.owner_installation === installation;
}

function isBot(req) {
  const ua = (req.headers.get("user-agent") || "").toLowerCase();
  if (!ua) return true;
  return /bot|crawl|spider|slurp|curl|wget|python-requests|headlesschrome|preview/.test(ua);
}

export default {
  async fetch(request, env) {
    return handle(request, env);
  },
};

async function handle(request, env) {
  const url = new URL(request.url);
    const method = request.method.toUpperCase();
    const db = env.DB; // D1 binding — CONFIRM production binding name

    // ===== health =====
    if (method === "GET" && url.pathname === "/health") {
      let database = false;
      try { await db.prepare("SELECT 1").first(); database = true; } catch { database = false; }
      return json({ service: "OBR Track", status: "ok", database });
    }

    // ===== public redirect =====
    const tMatch = url.pathname.match(/^\/t\/([A-Za-z0-9_-]+)$/);
    if (method === "GET" && tMatch) {
      const code = tMatch[1];
      if (!rateLimit(`tip:${clientIp(request)}`, 120, 60_000)) {
        return json({ error: "rate_limited" }, 429);
      }
      const link = await db.prepare(
        "SELECT tracking_code, original_ad_url, enabled FROM tracked_links WHERE tracking_code = ?"
      ).bind(code).first();
      if (!link) return json({ error: "not_found" }, 404);
      if (!link.enabled) return json({ error: "link_disabled" }, 410);

      const is_bot = isBot(request) ? 1 : 0;
      const vh = await visitorHashOf(request);
      if (!is_bot) {
        // dedup: same visitor within window is recorded but not counted
        const recent = await db.prepare(
          `SELECT id FROM click_events WHERE tracking_code = ? AND visitor_hash = ? AND is_bot = 0
           AND counted_raw = 1 AND created_at >= datetime('now', '-${DEDUP_SECONDS} seconds') LIMIT 1`
        ).bind(code, vh).first();
        const counted = recent ? 0 : 1;
        await db.prepare(
          `INSERT INTO click_events (tracking_code, visitor_hash, counted_raw, is_bot, created_at)
           VALUES (?, ?, ?, ?, datetime('now'))`
        ).bind(code, vh, counted, is_bot).run();
      }
      return Response.redirect(link.original_ad_url, 302);
    }

    // ===== v2: create link (server-generated code, no client secret) =====
    if (method === "POST" && url.pathname === "/v2/links") {
      let body;
      try { body = await request.json(); } catch { return json({ error: "invalid_json" }, 400); }
      const dest = String(body.destination_url || body.original_ad_url || "").trim();
      const installation = String(body.installation_id || "").trim();
      const product = String(body.product_id || "").trim();
      const hInst = (request.headers.get("x-obr-installation") || "").trim();
      const hProd = (request.headers.get("x-obr-product") || "").trim();

      if (!rateLimit(`ip:${clientIp(request)}`, 30, 60_000)) return json({ error: "rate_limited" }, 429);
      if (!INSTALLATION_RE.test(installation) || installation !== hInst) return json({ error: "installation_id غير صالح" }, 400);
      if (!PRODUCT_RE.test(product) || product !== hProd) return json({ error: "product_id غير صالح" }, 400);
      if (!rateLimit(`inst:${installation}`, 60, 60_000)) return json({ error: "rate_limited" }, 429);
      if (!/^https:\/\//.test(dest)) return json({ error: "destination_url يجب أن يكون https://" }, 400);

      const cnt = await db.prepare(
        "SELECT COUNT(*) AS n FROM tracked_links WHERE owner_installation = ?"
      ).bind(installation).first();
      if ((cnt?.n || 0) >= MAX_LINKS_PER_INSTALLATION) return json({ error: "quota_exceeded" }, 429);

      const code = [...crypto.getRandomValues(new Uint8Array(24))]
        .map((b) => b.toString(16).padStart(2, "0")).join("");
      await db.prepare(
        `INSERT INTO tracked_links (tracking_code, original_ad_url, enabled, owner_installation, created_at, updated_at)
         VALUES (?, ?, 1, ?, datetime('now'), datetime('now'))`
      ).bind(code, dest, installation).run();
      return json({ ok: true, tracking_code: code }, 201);
    }

    // ===== v2: patch (owner only) =====
    const pMatch = url.pathname.match(/^\/v2\/links\/([a-f0-9]{48})$/i);
    if (method === "PATCH" && pMatch) {
      const code = pMatch[1];
      if (!rateLimit(`ip:${clientIp(request)}`, 120, 60_000)) return json({ error: "rate_limited" }, 429);
      const installation = (request.headers.get("x-obr-installation") || "").trim();
      const link = await db.prepare("SELECT * FROM tracked_links WHERE tracking_code = ?").bind(code).first();
      if (!link) return json({ error: "الرابط غير موجود" }, 404);
      if (!v2Owns(link, installation)) return json({ error: "غير مصرح لهذا التثبيت" }, 403);
      let body;
      try { body = await request.json(); } catch { return json({ error: "invalid_json" }, 400); }
      if (body.original_ad_url != null && !/^https:\/\//.test(String(body.original_ad_url))) {
        return json({ error: "original_ad_url يجب أن يكون https://" }, 400);
      }
      const newUrl = body.original_ad_url != null ? String(body.original_ad_url) : link.original_ad_url;
      const newEnabled = body.enabled == null ? (link.enabled ? 1 : 0) : (body.enabled ? 1 : 0);
      await db.prepare(
        "UPDATE tracked_links SET original_ad_url = ?, enabled = ?, updated_at = datetime('now') WHERE tracking_code = ?"
      ).bind(newUrl, newEnabled, code).run();
      return json({ ok: true });
    }

    // ===== v2: stats (owner only) =====
    const sMatch = url.pathname.match(/^\/v2\/links\/([a-f0-9]{48})\/stats$/i);
    if (method === "GET" && sMatch) {
      const code = sMatch[1];
      if (!rateLimit(`ip:${clientIp(request)}`, 240, 60_000)) return json({ error: "rate_limited" }, 429);
      const installation = (request.headers.get("x-obr-installation") || "").trim();
      const link = await db.prepare("SELECT * FROM tracked_links WHERE tracking_code = ?").bind(code).first();
      if (!link) return json({ error: "الرابط غير موجود" }, 404);
      if (!v2Owns(link, installation)) return json({ error: "غير مصرح لهذا التثبيت" }, 403);
      const tracked = await db.prepare(
        "SELECT COUNT(*) AS n FROM click_events WHERE tracking_code = ? AND is_bot = 0 AND counted_raw = 1"
      ).bind(code).first();
      const unique = await db.prepare(
        "SELECT COUNT(DISTINCT visitor_hash) AS n FROM click_events WHERE tracking_code = ? AND is_bot = 0 AND counted_raw = 1"
      ).bind(code).first();
      const last = await db.prepare(
        "SELECT created_at FROM click_events WHERE tracking_code = ? AND is_bot = 0 AND counted_raw = 1 ORDER BY id DESC LIMIT 1"
      ).bind(code).first();
      return json({
        tracked_clicks: tracked?.n || 0,
        unique_clicks: unique?.n || 0,
        last_click_at: last?.created_at || null,
        enabled: !!link.enabled,
      });
    }

    return json({ error: "not_found" }, 404);
}

