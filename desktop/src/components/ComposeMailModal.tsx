import React, { useState } from 'react';
import { Mail } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { buildMailtoUrl, isAllowedExternalUrl, SUPPORT_EMAIL } from '../mail';

type Props = {
  onClose: () => void;
};

export default function ComposeMailModal({ onClose }: Props) {
  const [subject, setSubject] = useState('');
  const [body, setBody] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');

  const send = async () => {
    setError('');
    if (!body.trim() && !subject.trim()) {
      setError('اكتب موضوعًا أو نص الرسالة قبل الفتح');
      return;
    }
    let url: string;
    try {
      url = buildMailtoUrl(SUPPORT_EMAIL, subject, body);
    } catch (err: any) {
      setError(err?.message || 'تعذر تجهيز الرسالة');
      return;
    }
    if (!isAllowedExternalUrl(url)) {
      setError('رابط غير مسموح');
      return;
    }
    setBusy(true);
    try {
      await invoke('open_external_url', { url });
      onClose();
    } catch (err: any) {
      const message = typeof err === 'string' ? err : err?.message || 'تعذر فتح برنامج البريد';
      try {
        window.location.href = url;
        onClose();
      } catch {
        setError(message);
      }
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="modal-overlay" onClick={onClose} role="presentation">
      <div
        className="modal mail-compose-modal"
        role="dialog"
        aria-labelledby="mail-compose-title"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="modal-header">
          <h2 id="mail-compose-title" className="modal-title">
            <Mail size={17} strokeWidth={1.8} aria-hidden style={{ verticalAlign: '-3px', color: 'var(--gold)' }} />{' '}
            لوحة المراسلة
          </h2>
          <button type="button" className="modal-close" onClick={onClose} aria-label="إغلاق">
            ×
          </button>
        </div>
        <div className="modal-body">
          <label className="mail-compose-field">
            <span>إلى</span>
            <input value={SUPPORT_EMAIL} readOnly dir="ltr" />
          </label>
          <label className="mail-compose-field">
            <span>الموضوع</span>
            <input
              value={subject}
              onChange={(e) => setSubject(e.target.value)}
              placeholder="موضوع الرسالة"
            />
          </label>
          <label className="mail-compose-field">
            <span>الرسالة</span>
            <textarea
              value={body}
              onChange={(e) => setBody(e.target.value)}
              rows={7}
              placeholder="اكتب رسالتك هنا"
            />
          </label>
          {error ? <div className="error-alert">{error}</div> : null}
          <p className="mail-compose-note">
            يُفتح برنامج البريد الافتراضي على جهازك مع العنوان والنص. الإرسال يتم من ذلك البرنامج وليس من داخل التطبيق.
          </p>
        </div>
        <div className="modal-footer">
          <button type="button" className="btn btn-outline" onClick={onClose} disabled={busy}>
            إلغاء
          </button>
          <button type="button" className="btn btn-primary" onClick={send} disabled={busy}>
            {busy ? 'جارٍ الفتح...' : 'فتح برنامج البريد'}
          </button>
        </div>
      </div>
    </div>
  );
}
