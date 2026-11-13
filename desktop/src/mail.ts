export const SUPPORT_EMAIL = 'OBR66@outlook.com';
/** الموقع الرسمي لـ OBR */
export const SITE_URL = 'https://obr.obr432.workers.dev/';

const EMAIL_RE = /^[^\s@<>]+@[^\s@<>]+\.[^\s@<>]+$/;

export function buildMailtoUrl(to: string, subject: string, body: string): string {
  const dest = to.trim();
  if (!EMAIL_RE.test(dest)) {
    throw new Error('عنوان البريد غير صالح');
  }
  const q = new URLSearchParams();
  if (subject.trim()) q.set('subject', subject.trim());
  if (body.trim()) q.set('body', body.trim());
  const qs = q.toString();
  return qs ? `mailto:${dest}?${qs}` : `mailto:${dest}`;
}

export function isAllowedExternalUrl(url: string): boolean {
  const trimmed = url.trim();
  if (trimmed.includes('\n') || trimmed.includes('\r') || trimmed.includes('\0')) return false;
  return trimmed.startsWith('mailto:') || trimmed.startsWith('https://');
}
