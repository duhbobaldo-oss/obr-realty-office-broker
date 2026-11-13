const BOT_UA = [
  'bot', 'crawl', 'spider', 'slurp', 'facebookexternalhit', 'facebot',
  'twitterbot', 'linkedinbot', 'slackbot', 'discordbot', 'telegrambot',
  'whatsapp', 'preview', 'embedly', 'quora link preview', 'outbrain',
  'pinterest', 'vkshare', 'w3c_validator', 'whatsapp', 'googlebot',
  'bingpreview', 'yandex', 'baiduspider', 'duckduckbot', 'applebot',
  'semrush', 'ahrefs', 'mj12bot', 'dotbot', 'petalbot', 'bytespider'
];

export function isLikelyBotOrPreview(userAgent = '', headers = {}) {
  const ua = String(userAgent || '').toLowerCase();
  if (!ua) return true;
  if (BOT_UA.some((p) => ua.includes(p))) return true;

  const purpose = String(headers['purpose'] || headers['x-purpose'] || '').toLowerCase();
  if (purpose === 'prefetch' || purpose === 'preview') return true;

  const secPurpose = String(headers['sec-fetch-purpose'] || '').toLowerCase();
  if (secPurpose === 'prefetch' || secPurpose === 'preview') return true;

  const dest = String(headers['sec-fetch-dest'] || '').toLowerCase();
  if (dest === 'serviceworker') return true;

  return false;
}
