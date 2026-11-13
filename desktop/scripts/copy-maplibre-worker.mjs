// Copies maplibre-gl's worker + shared chunks next to the bundle output.
// maplibre-gl v6 resolves its worker URL relative to the bundle:
//   new URL('./maplibre-gl-worker.mjs', import.meta.url)
// Vite does not emit these files, so without this step the installed app
// 404s the worker (SPA fallback returns text/html) and the map renders blank.
import { copyFileSync, mkdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';

const root = dirname(dirname(fileURLToPath(import.meta.url)));
// node_modules قد يكون مرفوعًا لجذر المستودع (npm workspaces) — الحل عبر require.resolve
const require = createRequire(join(root, 'package.json'));
const pkgDir = dirname(require.resolve('maplibre-gl/package.json'));
const src = join(pkgDir, 'dist');
const dest = join(root, 'public', 'assets');
mkdirSync(dest, { recursive: true });

const files = ['maplibre-gl-worker.mjs', 'maplibre-gl-shared.mjs'];
for (const f of files) {
  copyFileSync(join(src, f), join(dest, f));
  console.log('[copy-maplibre]', f, '→ public/assets/');
}
