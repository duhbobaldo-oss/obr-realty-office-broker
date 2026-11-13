import http from 'node:http';
import crypto from 'node:crypto';
import { URL } from 'node:url';
import { isLikelyBotOrPreview } from './bots.mjs';
import {
  countLinksByOwner,
  getLink,
  getStats,
  openDb,
  patchLink,
  recordClick,
  upsertLink,
  visitorHash
} from './store.mjs';

const PORT = Number(process.env.PORT || process.env.OBR_TRACKING_PORT || 8787);
const HOST = process.env.HOST || '0.0.0.0';
const db = openDb();

// ===== v2: بروتوكول بلا مفتاح (Desktop = untrusted public client) =====
// التطبيق يحدد نفسه بـ X-OBR-Installation + X-OBR-Product (تحديد وليس مصادقة).
// الخادم يملك التحقق من المدخلات، حدود المعدل، والحصص لكل تثبيت.
const V2_INSTALLATION_RE = /^[A-Za-z0-9_-]{1,64}$/;
const V2_PRODUCT_RE = /^[a-z0-9-]{1,64}$/;
const V2_CODE_RE = /^[a-f0-9]{48}$/;
const V2_MAX_LINKS_PER_INSTALLATION = 5000;

function v2Code() {
  return crypto.randomBytes(24).toString('hex');
}

function installationHeader(req) {
  const v = req.headers['x-obr-installation'];
  return typeof v === 'string' ? v.trim() : '';
}

function productHeader(req) {
  const v = req.headers['x-obr-product'];
  return typeof v === 'string' ? v.trim() : '';
}

/** حد معدل بسيط في الذاكرة: مفتاح → أزمنة داخل النافذة. */
const rateBuckets = new Map();
function rateLimit(key, max, windowMs) {
  const now = Date.now();
  let arr = rateBuckets.get(key);
  if (!arr) {
    arr = [];
    rateBuckets.set(key, arr);
  }
  while (arr.length && now - arr[0] > windowMs) arr.shift();
  if (arr.length >= max) return false;
  arr.push(now);
  return true;
}

/** ملكية الرابط: روابط v2 مملوكة للتثبيت المنشئ؛ روابط v1 (بلا مالك) متاحة. */
function v2OwnsLink(req, link) {
  const owner = link.owner_installation;
  if (!owner) return true;
  return owner === installationHeader(req);
}

function readJson(req) {
  return new Promise((resolve, reject) => {
    const chunks = [];
    req.on('data', (c) => chunks.push(c));
    req.on('end', () => {
      if (!chunks.length) return resolve({});
      try {
        resolve(JSON.parse(Buffer.concat(chunks).toString('utf8')));
      } catch (e) {
        reject(new Error('JSON غير صالح'));
      }
    });
    req.on('error', reject);
  });
}

function sendJson(res, status, body) {
  const raw = JSON.stringify(body);
  res.writeHead(status, {
    'Content-Type': 'application/json; charset=utf-8',
    'Cache-Control': 'no-store'
  });
  res.end(raw);
}

function clientIp(req) {
  const xf = req.headers['x-forwarded-for'];
  if (typeof xf === 'string' && xf.trim()) return xf.split(',')[0].trim();
  return req.socket?.remoteAddress || '';
}

function isHttpsUrl(value) {
  try {
    const u = new URL(String(value || ''));
    return u.protocol === 'https:';
  } catch {
    return false;
  }
}

const server = http.createServer(async (req, res) => {
  try {
    const host = req.headers.host || `127.0.0.1:${PORT}`;
    const url = new URL(req.url || '/', `http://${host}`);
    const method = (req.method || 'GET').toUpperCase();

    if (method === 'GET' && url.pathname === '/health') {
      return sendJson(res, 200, { ok: true, service: 'obr-campaign-tracking' });
    }

    if (method === 'POST' && url.pathname === '/v1/links') {
      const body = await readJson(req);
      const code = String(body.tracking_code || '').trim();
      const dest = String(body.original_ad_url || '').trim();
      if (!/^[a-f0-9]{32,64}$/i.test(code)) {
        return sendJson(res, 400, { error: 'tracking_code غير صالح' });
      }
      if (!isHttpsUrl(dest)) {
        return sendJson(res, 400, { error: 'original_ad_url يجب أن يكون https://' });
      }
      upsertLink(db, { tracking_code: code, original_ad_url: dest, enabled: true });
      return sendJson(res, 200, { ok: true, tracking_code: code });
    }

    const patchMatch = url.pathname.match(/^\/v1\/links\/([a-f0-9]{32,64})$/i);
    if (method === 'PATCH' && patchMatch) {
      const code = patchMatch[1];
      const body = await readJson(req);
      if (body.original_ad_url != null && !isHttpsUrl(body.original_ad_url)) {
        return sendJson(res, 400, { error: 'original_ad_url يجب أن يكون https://' });
      }
      const updated = patchLink(db, code, {
        original_ad_url: body.original_ad_url,
        enabled: body.enabled
      });
      if (!updated) return sendJson(res, 404, { error: 'الرابط غير موجود' });
      return sendJson(res, 200, { ok: true });
    }

    const statsMatch = url.pathname.match(/^\/v1\/links\/([a-f0-9]{32,64})\/stats$/i);
    if (method === 'GET' && statsMatch) {
      const stats = getStats(db, statsMatch[1]);
      if (!stats) return sendJson(res, 404, { error: 'الرابط غير موجود' });
      // لا نُرجع IP أو User-Agent أبدًا
      return sendJson(res, 200, stats);
    }

    const clickMatch = url.pathname.match(/^\/t\/([a-f0-9]{32,64})$/i);
    if ((method === 'GET' || method === 'HEAD') && clickMatch) {
      const code = clickMatch[1];
      const link = getLink(db, code);
      const fallback = link?.original_ad_url;
      if (!link || !fallback) {
        res.writeHead(404, { 'Content-Type': 'text/plain; charset=utf-8' });
        return res.end('رابط التتبع غير موجود');
      }

      // حتى لو معطّل التتبع: لا نمنع فتح الوجهة الأصلية
      const ua = req.headers['user-agent'] || '';
      const bot = isLikelyBotOrPreview(ua, req.headers);
      if (link.enabled) {
        const hash = visitorHash(clientIp(req), ua);
        recordClick(db, { tracking_code: code, visitor_hash: hash, is_bot: bot });
      }

      res.writeHead(302, {
        Location: fallback,
        'Cache-Control': 'no-store, no-cache, must-revalidate',
        'Referrer-Policy': 'no-referrer'
      });
      return res.end();
    }

    // ===== v2: إنشاء رابط — الرمز يولَّد على الخادم (لا مفتاح من العميل) =====
    if (method === 'POST' && url.pathname === '/v2/links') {
      const body = await readJson(req);
      const dest = String(body.destination_url || body.original_ad_url || '').trim();
      const installation = String(body.installation_id || '').trim();
      const product = String(body.product_id || '').trim();

      if (!rateLimit(`ip:${clientIp(req)}`, 30, 60_000)) {
        return sendJson(res, 429, { error: 'عدد كبير من الطلبات، حاول لاحقًا' });
      }
      if (!V2_INSTALLATION_RE.test(installation) || installation !== installationHeader(req)) {
        return sendJson(res, 400, { error: 'installation_id غير صالح' });
      }
      if (!V2_PRODUCT_RE.test(product) || product !== productHeader(req)) {
        return sendJson(res, 400, { error: 'product_id غير صالح' });
      }
      if (!rateLimit(`inst:${installation}`, 60, 60_000)) {
        return sendJson(res, 429, { error: 'عدد كبير من الطلبات لهذا التثبيت' });
      }
      if (!isHttpsUrl(dest)) {
        return sendJson(res, 400, { error: 'destination_url يجب أن يكون https://' });
      }
      if (countLinksByOwner(db, installation) >= V2_MAX_LINKS_PER_INSTALLATION) {
        return sendJson(res, 429, { error: 'تم بلوغ الحد الأقصى لروابط هذا التثبيت' });
      }

      const code = v2Code();
      upsertLink(db, { tracking_code: code, original_ad_url: dest, enabled: true, owner_installation: installation });
      return sendJson(res, 201, { ok: true, tracking_code: code });
    }

    // ===== v2: تحديث/تعطيل — للمالك فقط =====
    const v2PatchMatch = url.pathname.match(/^\/v2\/links\/([a-f0-9]{48})$/i);
    if (method === 'PATCH' && v2PatchMatch) {
      const code = v2PatchMatch[1];
      if (!rateLimit(`ip:${clientIp(req)}`, 120, 60_000)) {
        return sendJson(res, 429, { error: 'عدد كبير من الطلبات، حاول لاحقًا' });
      }
      const link = getLink(db, code);
      if (!link) return sendJson(res, 404, { error: 'الرابط غير موجود' });
      if (!v2OwnsLink(req, link)) return sendJson(res, 403, { error: 'غير مصرح لهذا التثبيت' });
      const body = await readJson(req);
      if (body.original_ad_url != null && !isHttpsUrl(body.original_ad_url)) {
        return sendJson(res, 400, { error: 'original_ad_url يجب أن يكون https://' });
      }
      patchLink(db, code, {
        original_ad_url: body.original_ad_url,
        enabled: body.enabled
      });
      return sendJson(res, 200, { ok: true });
    }

    // ===== v2: الإحصاءات — للمالك فقط =====
    const v2StatsMatch = url.pathname.match(/^\/v2\/links\/([a-f0-9]{48})\/stats$/i);
    if (method === 'GET' && v2StatsMatch) {
      const code = v2StatsMatch[1];
      if (!rateLimit(`ip:${clientIp(req)}`, 240, 60_000)) {
        return sendJson(res, 429, { error: 'عدد كبير من الطلبات، حاول لاحقًا' });
      }
      const link = getLink(db, code);
      if (!link) return sendJson(res, 404, { error: 'الرابط غير موجود' });
      if (!v2OwnsLink(req, link)) return sendJson(res, 403, { error: 'غير مصرح لهذا التثبيت' });
      // لا نُرجع IP أو User-Agent أبدًا
      return sendJson(res, 200, getStats(db, code));
    }

    sendJson(res, 404, { error: 'not found' });
  } catch (err) {
    sendJson(res, 500, { error: err?.message || 'server error' });
  }
});

server.listen(PORT, HOST, () => {
  console.log(`[obr-tracking] listening on http://${HOST}:${PORT}`);
});
