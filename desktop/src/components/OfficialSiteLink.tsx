import React from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Globe2 } from 'lucide-react';
import { isAllowedExternalUrl, SITE_URL } from '../mail';

type Props = {
  className?: string;
};

export default function OfficialSiteLink({ className = '' }: Props) {
  const openSite = () => {
    if (!isAllowedExternalUrl(SITE_URL)) return;
    void invoke('open_external_url', { url: SITE_URL });
  };

  return (
    <button type="button" className={className} dir="rtl" onClick={openSite}>
      <Globe2 size={12} strokeWidth={1.8} aria-hidden="true" />
      <span>الموقع الرسمي</span>
    </button>
  );
}
