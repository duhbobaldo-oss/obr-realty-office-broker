import assert from 'node:assert/strict';
import test from 'node:test';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { isLikelyBotOrPreview } from '../src/bots.mjs';
import {
  getStats,
  openDb,
  recordClick,
  upsertLink,
  visitorHash
} from '../src/store.mjs';

test('bot and preview detection', () => {
  assert.equal(isLikelyBotOrPreview('Mozilla/5.0'), false);
  assert.equal(isLikelyBotOrPreview('facebookexternalhit/1.1'), true);
  assert.equal(isLikelyBotOrPreview('Twitterbot/1.0'), true);
  assert.equal(isLikelyBotOrPreview('Mozilla/5.0', { purpose: 'prefetch' }), true);
});

test('raw clicks vs unique and rapid dedup', () => {
  const dbPath = path.join(os.tmpdir(), `obr-track-test-${Date.now()}.sqlite`);
  const db = openDb(dbPath);
  upsertLink(db, {
    tracking_code: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
    original_ad_url: 'https://sa.bayut.com/property/x',
    enabled: true
  });
  const code = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';
  const a = visitorHash('1.1.1.1', 'Mozilla/5.0 Chrome');
  const b = visitorHash('2.2.2.2', 'Mozilla/5.0 Firefox');

  assert.equal(recordClick(db, { tracking_code: code, visitor_hash: a, is_bot: false }).counted, true);
  assert.equal(recordClick(db, { tracking_code: code, visitor_hash: a, is_bot: false }).counted, false);
  assert.equal(recordClick(db, { tracking_code: code, visitor_hash: b, is_bot: false }).counted, true);
  assert.equal(recordClick(db, { tracking_code: code, visitor_hash: a, is_bot: true }).counted, false);

  const stats = getStats(db, code);
  assert.equal(stats.tracked_clicks, 2);
  assert.equal(stats.unique_clicks, 2);

  db.close();
  fs.unlinkSync(dbPath);
});
