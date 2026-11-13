import { invoke } from '@tauri-apps/api/core';
import * as maplibregl from 'maplibre-gl';
import { PMTiles, type Source } from 'pmtiles';

class TauriRangeSource implements Source {
  constructor(private assetId: string) {}

  getKey() {
    return `obr-pmtiles:${this.assetId}`;
  }

  async getBytes(offset: number, length: number): Promise<{ data: ArrayBuffer }> {
    const raw = await invoke<number[] | Uint8Array>('map_read_range', {
      assetId: this.assetId,
      offset,
      length
    });
    const bytes = raw instanceof Uint8Array ? raw : Uint8Array.from(raw);
    const copy = new Uint8Array(bytes.byteLength);
    copy.set(bytes);
    return { data: copy.buffer };
  }
}

const archives = new Map<string, PMTiles>();
let protocolBound = false;

export function getLocalArchive(assetId: string): PMTiles {
  const existing = archives.get(assetId);
  if (existing) return existing;
  const tiles = new PMTiles(new TauriRangeSource(assetId));
  archives.set(assetId, tiles);
  return tiles;
}

export function bindLocalTileProtocol() {
  if (protocolBound) return;
  maplibregl.addProtocol('obrpm', async (params: { url: string }, abort: AbortController) => {
    const match = params.url.match(/^obrpm:\/\/([^/]+)\/(\d+)\/(\d+)\/(\d+)/);
    if (!match) {
      throw new Error('عنوان بلاطة غير صالح');
    }
    const assetId = match[1];
    const z = Number(match[2]);
    const x = Number(match[3]);
    const y = Number(match[4]);
    const archive = getLocalArchive(assetId);
    const tile = await archive.getZxy(z, x, y, abort.signal);
    if (!tile) return { data: new Uint8Array() };
    return { data: new Uint8Array(tile.data) };
  });
  protocolBound = true;
}
