import { t as systemT } from '../domain/locale';
import { invoke } from '@tauri-apps/api/core';
import { getUiLanguage } from '../domain/locale';
import { save } from '@tauri-apps/plugin-dialog';

export type DesignExportFormat = 'pdf' | 'doc' | 'png' | 'html';

const FORMAT_META: Record<
  DesignExportFormat,
  { label: string; extensions: string[]; filterName: string }
> = {
  pdf: { label: 'PDF', extensions: ['pdf'], filterName: 'PDF' },
  doc: { label: 'Word', extensions: ['doc'], filterName: 'Word' },
  png: { get label() { return systemT('design.ui.118'); }, extensions: ['png'], filterName: 'PNG' },
  html: { label: 'HTML', extensions: ['html'], filterName: 'HTML' }
};

export function designExportFormats(): { id: DesignExportFormat; label: string }[] {
  return (Object.keys(FORMAT_META) as DesignExportFormat[])
    .filter((id) => id !== 'html')
    .map((id) => ({ id, label: FORMAT_META[id].label }));
}

function safeStem(title: string): string {
  return (title || systemT('design.ui.165')).replace(/[\\/:*?"<>|]/g, '_').trim() || systemT('design.ui.165');
}

/** HTML متوافق مع فتح Word مباشرة كملف .doc */
export function wrapWordHtml(title: string, bodyInner: string, pageCssSize: string, extraCss: string): string {
  return `<!DOCTYPE html>
<html xmlns:o="urn:schemas-microsoft-com:office:office"
 xmlns:w="urn:schemas-microsoft-com:office:word"
 xmlns="http://www.w3.org/TR/REC-html40" lang="${getUiLanguage()}" dir="${getUiLanguage() === 'ar' ? 'rtl' : 'ltr'}">
<head>
<meta charset="utf-8" />
<meta http-equiv="Content-Type" content="text/html; charset=utf-8" />
<title>${escapeHtml(title)}</title>
<!--[if gte mso 9]><xml>
 <w:WordDocument>
  <w:View>Print</w:View>
  <w:Zoom>100</w:Zoom>
  <w:DoNotOptimizeForBrowser/>
 </w:WordDocument>
</xml><![endif]-->
<style>
  @page { size: ${pageCssSize}; margin: 0; }
  * { box-sizing: border-box; }
  body { margin: 0; font-family: "IBM Plex Sans Arabic", "Segoe UI", Tahoma, Arial, sans-serif; background: #fff; }
  img { max-width: 100%; display: block; }
  ${extraCss}
</style>
</head>
<body>${bodyInner}</body>
</html>`;
}

export function wrapPrintHtml(title: string, bodyInner: string, pageCssSize: string, extraCss: string): string {
  return `<!DOCTYPE html>
<html lang="${getUiLanguage()}" dir="${getUiLanguage() === 'ar' ? 'rtl' : 'ltr'}">
<head>
<meta charset="utf-8" />
<title>${escapeHtml(title)}</title>
<style>
  @page { size: ${pageCssSize}; margin: 0; }
  * { box-sizing: border-box; }
  html, body { margin: 0; padding: 0; background: #fff; }
  body { font-family: "IBM Plex Sans Arabic", "Segoe UI Variable", "Segoe UI", Tahoma, Arial, sans-serif; }
  img { max-width: 100%; display: block; object-fit: contain; }
  ${extraCss}
  @media print { body { -webkit-print-color-adjust: exact; print-color-adjust: exact; } }
</style>
</head>
<body>${bodyInner}</body>
</html>`;
}

function escapeHtml(value: string): string {
  return value
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}

export async function exportDesignDocument(opts: {
  format: DesignExportFormat;
  title: string;
  bodyHtml: string;
  pageCssSize: string;
  extraCss: string;
  widthPx: number;
  heightPx: number;
}): Promise<{ path: string } | { cancelled: true }> {
  const meta = FORMAT_META[opts.format];
  const html =
    opts.format === 'doc'
      ? wrapWordHtml(opts.title, opts.bodyHtml, opts.pageCssSize, opts.extraCss)
      : wrapPrintHtml(opts.title, opts.bodyHtml, opts.pageCssSize, opts.extraCss);

  const path = await save({
    defaultPath: `${safeStem(opts.title)}.${meta.extensions[0]}`,
    filters: [{ name: meta.filterName, extensions: meta.extensions }]
  });
  if (!path) return { cancelled: true };

  let token = '';
  try {
    const saved = localStorage.getItem('mawasib_auth');
    if (saved) {
      const parsed = JSON.parse(saved) as { token?: string };
      token = parsed.token || '';
    }
  } catch {
    token = '';
  }
  if (!token) {
    throw new Error(systemT('design.ui.166'));
  }

  await invoke('export_design_document', {
    token,
    html,
    format: opts.format,
    destPath: path,
    widthPx: Math.round(opts.widthPx),
    heightPx: Math.round(opts.heightPx)
  });
  return { path };
}
