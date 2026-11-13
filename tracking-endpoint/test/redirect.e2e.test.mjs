import assert from 'node:assert/strict';
import test from 'node:test';
import http from 'node:http';
import { spawn } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.join(__dirname, '..');
const PORT = 8799;
const CODE = 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb';
const DEST = 'https://sa.bayut.com/property/test-redirect';

function request(method, urlPath, { headers = {}, body } = {}) {
  return new Promise((resolve, reject) => {
    const req = http.request(
      {
        hostname: '127.0.0.1',
        port: PORT,
        path: urlPath,
        method,
        headers: {
          ...(body ? { 'Content-Type': 'application/json', 'Content-Length': Buffer.byteLength(body) } : {}),
          ...headers
        }
      },
      (res) => {
        const chunks = [];
        res.on('data', (c) => chunks.push(c));
        res.on('end', () => {
          resolve({
            status: res.statusCode,
            headers: res.headers,
            body: Buffer.concat(chunks).toString('utf8')
          });
        });
      }
    );
    req.on('error', reject);
    if (body) req.write(body);
    req.end();
  });
}

test('live redirect + 10 human clicks + bot + dedup', async (t) => {
  const dbPath = path.join(os.tmpdir(), `obr-track-e2e-${Date.now()}.sqlite`);
  const child = spawn(process.execPath, ['src/server.mjs'], {
    cwd: root,
    env: { ...process.env, PORT: String(PORT), OBR_TRACKING_DB: dbPath },
    stdio: ['ignore', 'pipe', 'pipe']
  });
  t.after(() => {
    child.kill();
    try { fs.unlinkSync(dbPath); } catch { /* ignore */ }
  });

  await new Promise((r) => setTimeout(r, 600));

  const reg = await request('POST', '/v1/links', {
    body: JSON.stringify({ tracking_code: CODE, original_ad_url: DEST })
  });
  assert.equal(reg.status, 200);

  for (let i = 0; i < 10; i++) {
    const click = await request('GET', `/t/${CODE}`, {
      headers: { 'User-Agent': `Mozilla/5.0 TestClient/${i}`, 'X-Forwarded-For': `203.0.113.${i + 1}` }
    });
    assert.equal(click.status, 302);
    assert.equal(click.headers.location, DEST);
  }

  const bot = await request('GET', `/t/${CODE}`, {
    headers: { 'User-Agent': 'facebookexternalhit/1.1', 'X-Forwarded-For': '198.51.100.9' }
  });
  assert.equal(bot.status, 302);

  const dedup = await request('GET', `/t/${CODE}`, {
    headers: { 'User-Agent': 'Mozilla/5.0 TestClient/0', 'X-Forwarded-For': '203.0.113.1' }
  });
  assert.equal(dedup.status, 302);

  const stats = JSON.parse((await request('GET', `/v1/links/${CODE}/stats`)).body);
  assert.equal(stats.tracked_clicks, 10);
  assert.equal(stats.unique_clicks, 10);
  assert.ok(stats.last_click_at);
});
