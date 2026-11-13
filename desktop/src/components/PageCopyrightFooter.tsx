import React from 'react';
import OfficialSiteLink from './OfficialSiteLink';

type Props = {
  className?: string;
};

/** تذييل ثابت أسفل كل الأقسام — رابط الموقع الرسمي. */
export default function PageCopyrightFooter({ className = '' }: Props) {
  return (
    <footer className={`page-copyright-footer ${className}`.trim()}>
      <OfficialSiteLink className="page-copyright-email" />
    </footer>
  );
}
