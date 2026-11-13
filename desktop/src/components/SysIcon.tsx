import React from 'react';

const modules = import.meta.glob('../assets/icons/system-uicons/*.svg', {
  eager: true,
  query: '?url',
  import: 'default'
}) as Record<string, string>;

function resolve(name: string): string | undefined {
  const key = Object.keys(modules).find((k) => k.replace(/\\/g, '/').endsWith(`/system-uicons/${name}.svg`));
  return key ? modules[key] : undefined;
}

/** أسماء system-uicons المعتمدة في الواجهة — مصدرها Design/icon. */
export type SysIconName =
  | 'home'
  | 'home-door'
  | 'users'
  | 'user'
  | 'user-circle'
  | 'briefcase'
  | 'document'
  | 'document-stack'
  | 'write'
  | 'pen'
  | 'graph-bar'
  | 'graph-increase'
  | 'database'
  | 'settings'
  | 'info-circle'
  | 'search'
  | 'plus'
  | 'plus-circle'
  | 'bell'
  | 'lock'
  | 'archive'
  | 'chevron-right'
  | 'chevron-left'
  | 'create'
  | 'list'
  | 'cross'
  | 'cross-circle'
  | 'warning-circle'
  | 'download'
  | 'upload'
  | 'printer'
  | 'location'
  | 'check'
  | 'check-circle'
  | 'box'
  | 'wallet'
  | 'credit-card'
  | 'coin'
  | 'coins'
  | 'target'
  | 'funnel'
  | 'calendar'
  | 'calendar-days'
  | 'clock'
  | 'mail'
  | 'camera'
  | 'tag'
  | 'tags'
  | 'filter'
  | 'share'
  | 'receipt'
  | 'eye'
  | 'folder-open'
  | 'clipboard'
  | 'clipboard-notes'
  | 'message'
  | 'calculator'
  | 'scale'
  | 'grid'
  | 'table'
  | 'book'
  | 'bookmark'
  | 'heart'
  | 'star'
  | 'inbox'
  | 'refresh'
  | 'trash'
  | 'duplicate'
  | 'link'
  | 'external'
  | 'files-stack'
  | 'postcard'
  | 'speaker';

interface Props {
  name: SysIconName;
  size?: number;
  className?: string;
  title?: string;
}

/** أيقونة system-uicons فقط — لغة بصرية واحدة للـ Shell */
export default function SysIcon({ name, size = 18, className = '', title }: Props) {
  const src = resolve(name);
  if (!src) {
    return (
      <span
        className={className}
        title={title || name}
        aria-hidden={!title}
        style={{ width: size, height: size, display: 'inline-block' }}
      />
    );
  }
  return (
    <img
      src={src}
      alt=""
      title={title}
      width={size}
      height={size}
      className={`sys-icon ${className}`}
      draggable={false}
      style={{ width: size, height: size, display: 'block' }}
    />
  );
}
