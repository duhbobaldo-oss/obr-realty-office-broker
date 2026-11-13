import React, { useEffect, useMemo, useRef, useState } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import * as maplibregl from 'maplibre-gl';
import 'maplibre-gl/dist/maplibre-gl.css';
import { MapPin, Pencil, Plus, Save, Trash2, Crosshair, MapPinned, CircleAlert } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { api } from '../api';
import { formatMoney, PROPERTY_STATUSES, propertyStatusLabel } from '../domain/labels';
import { hasCoords, isUsableMapPoint, listingPopupHtml, listingsToGeoJSON, type MapListing } from '../map/listings';
import { buildMapStyle, parseQueryPoint, RIYADH_CENTER } from '../map/style';
import SectionGuideBanner from '../components/SectionGuide';
import { useLocale } from '../hooks/useLocale';
import { t as tLocale } from '../domain/locale';

type PendingPoint = { lng: number; lat: number };

export default function PropertyMap() {
  const { t, tf } = useLocale();
  const navigate = useNavigate();
  const [params, setParams] = useSearchParams();
  const mapNode = useRef<HTMLDivElement>(null);
  const mapRef = useRef<maplibregl.Map | null>(null);
  const pendingMarker = useRef<maplibregl.Marker | null>(null);
  const hoverPopup = useRef<maplibregl.Popup | null>(null);

  const [items, setItems] = useState<MapListing[]>([]);
  const [loadError, setLoadError] = useState('');
  const [saveMsg, setSaveMsg] = useState('');
  const [saveErr, setSaveErr] = useState('');
  const [busy, setBusy] = useState(false);
  const [selectedId, setSelectedId] = useState(params.get('property') || '');
  const [pending, setPending] = useState<PendingPoint | null>(null);
  const [newName, setNewName] = useState('');

  const selected = items.find((p) => p.id === selectedId) || null;
  const located = useMemo(() => items.filter(hasCoords), [items]);
  const unlocated = useMemo(() => items.filter((p) => !hasCoords(p)), [items]);
  const itemsRef = useRef(items);
  itemsRef.current = items;

  const loadListings = async () => {
    setLoadError('');
    try {
      const data = await api('/api/properties?limit=1000&page=1');
      setItems(data.items || []);
    } catch (err: any) {
      setLoadError(err?.message || tLocale('maps.load-fail'));
    }
  };

  useEffect(() => {
    loadListings();
  }, []);

  useEffect(() => {
    const q = params.get('property');
    if (q) setSelectedId(q);
  }, [params]);

  useEffect(() => {
    if (!mapNode.current || mapRef.current) return;
    const map = new maplibregl.Map({
      container: mapNode.current,
      style: buildMapStyle(),
      center: RIYADH_CENTER,
      zoom: 11,
      minZoom: 4,
      // الخريطة نفسها تصل إلى 20؛ المصدر يقف عند 14 وبعده Overzoom فقط.
      maxZoom: 20,
      attributionControl: { compact: true }
    });
    map.addControl(new maplibregl.NavigationControl({ showCompass: false }), 'top-left');
    mapRef.current = map;

    map.on('load', () => {
      map.resize();
      map.getCanvas().style.cursor = 'crosshair';
      map.addSource('listings', {
        type: 'geojson',
        data: listingsToGeoJSON(itemsRef.current),
        cluster: true,
        clusterRadius: 42,
        clusterMaxZoom: 14
      });
      map.addLayer({
        id: 'listings-clusters',
        type: 'circle',
        source: 'listings',
        filter: ['has', 'point_count'],
        paint: {
          'circle-color': '#23365f',
          'circle-radius': ['step', ['get', 'point_count'], 16, 8, 20, 20, 26],
          'circle-stroke-width': 2,
          'circle-stroke-color': '#fff'
        }
      });
      map.addLayer({
        id: 'listings-points',
        type: 'circle',
        source: 'listings',
        filter: ['!', ['has', 'point_count']],
        paint: {
          'circle-color': [
            'match',
            ['get', 'status'],
            'available', '#059669',
            'negotiating', '#d97706',
            'reserved', '#23365f',
            'sold', '#dc2626',
            '#6f7380'
          ],
          'circle-radius': 10,
          'circle-stroke-width': [
            'case',
            ['==', ['get', 'id'], selectedId || ''],
            3,
            1.5
          ],
          'circle-stroke-color': '#fff'
        }
      });

      const queryPoint = parseQueryPoint(params);
      if (queryPoint) {
        setPending(queryPoint);
        map.jumpTo({ center: [queryPoint.lng, queryPoint.lat], zoom: Math.max(map.getZoom(), 14) });
      } else {
        map.jumpTo({ center: RIYADH_CENTER, zoom: 11 });
        setPending((p) => (p && Math.abs(p.lat) < 0.02 && Math.abs(p.lng) < 0.02 ? null : p));
      }
    });

    map.on('click', 'listings-clusters', (e: maplibregl.MapLayerMouseEvent) => {
      const feature = e.features?.[0];
      const clusterId = feature?.properties?.cluster_id;
      const source = map.getSource('listings') as maplibregl.GeoJSONSource;
      if (!feature || clusterId == null || !source.getClusterExpansionZoom) return;
      const coords = (feature.geometry as GeoJSON.Point).coordinates as [number, number];
      source.getClusterExpansionZoom(clusterId).then((zoom: number) => {
        map.easeTo({ center: coords, zoom });
      });
    });

    map.on('click', 'listings-points', (e: maplibregl.MapLayerMouseEvent) => {
      const id = e.features?.[0]?.properties?.id;
      if (!id) return;
      e.originalEvent.stopPropagation();
      setSelectedId(id);
      setParams({ property: id }, { replace: true });
    });

    map.on('click', (e: maplibregl.MapMouseEvent) => {
      const hits = map.queryRenderedFeatures(e.point, { layers: ['listings-points', 'listings-clusters'] });
      if (hits.length) return;
      const next = { lng: e.lngLat.lng, lat: e.lngLat.lat };
      setPending(next);
      setSaveErr('');
      setSaveMsg('');
    });

    const popup = new maplibregl.Popup({
      closeButton: false,
      closeOnClick: false,
      offset: 14,
      className: 'map-listing-popup',
      maxWidth: '280px',
      anchor: 'bottom'
    });
    hoverPopup.current = popup;

    const showListingCard = (e: maplibregl.MapLayerMouseEvent) => {
      const feature = e.features?.[0];
      if (!feature || feature.geometry.type !== 'Point') return;
      const coords = (feature.geometry as GeoJSON.Point).coordinates as [number, number];
      const props = (feature.properties || {}) as Record<string, unknown>;
      map.getCanvas().style.cursor = 'pointer';
      popup
        .setLngLat(coords)
        .setHTML(listingPopupHtml(props, propertyStatusLabel, formatMoney))
        .addTo(map);
    };

    map.on('mouseenter', 'listings-points', showListingCard);
    map.on('mousemove', 'listings-points', showListingCard);
    map.on('mouseleave', 'listings-points', () => {
      map.getCanvas().style.cursor = 'crosshair';
      popup.remove();
    });
    map.on('mouseenter', 'listings-clusters', () => { map.getCanvas().style.cursor = 'pointer'; });
    map.on('mouseleave', 'listings-clusters', () => { map.getCanvas().style.cursor = 'crosshair'; });

    return () => {
      popup.remove();
      hoverPopup.current = null;
      pendingMarker.current?.remove();
      map.remove();
      mapRef.current = null;
    };
  }, []);

  useEffect(() => {
    const map = mapRef.current;
    if (!map?.getSource('listings')) return;
    (map.getSource('listings') as maplibregl.GeoJSONSource).setData(listingsToGeoJSON(items));
  }, [items]);

  useEffect(() => {
    const map = mapRef.current;
    if (!map?.getLayer('listings-points')) return;
    map.setPaintProperty('listings-points', 'circle-stroke-width', [
      'case',
      ['==', ['get', 'id'], selectedId || ''],
      3,
      1.5
    ]);
  }, [selectedId]);

  useEffect(() => {
    pendingMarker.current?.remove();
    pendingMarker.current = null;
    const map = mapRef.current;
    if (!map || !pending) return;
    pendingMarker.current = new maplibregl.Marker({ color: '#d97706', draggable: true })
      .setLngLat([pending.lng, pending.lat])
      .addTo(map);
    pendingMarker.current.on('dragend', () => {
      const pos = pendingMarker.current?.getLngLat();
      if (!pos) return;
      setPending({ lng: pos.lng, lat: pos.lat });
    });
  }, [pending]);

  const flyToListing = (item: MapListing) => {
    if (!mapRef.current) return;
    if (!hasCoords(item)) {
      mapRef.current.easeTo({ center: RIYADH_CENTER, zoom: 11 });
      return;
    }
    mapRef.current.easeTo({
      center: [Number(item.longitude), Number(item.latitude)],
      zoom: Math.max(mapRef.current.getZoom(), 14)
    });
  };

  const requireUsablePending = () => {
    if (!pending) {
      setSaveErr(t('maps.click-city-first'));
      return false;
    }
    if (!isUsableMapPoint(pending.lat, pending.lng)) {
      setSaveErr(t('maps.ocean-point'));
      return false;
    }
    return true;
  };

  const selectListing = (id: string, fly = false) => {
    setSelectedId(id);
    setParams({ property: id }, { replace: true });
    const item = items.find((p) => p.id === id);
    if (fly && item) flyToListing(item);
  };

  const savePendingToListing = async () => {
    if (!selected || !requireUsablePending() || !pending) return;
    setBusy(true);
    setSaveErr('');
    setSaveMsg('');
    try {
      await api(`/api/properties/${selected.id}`, {
        method: 'PUT',
        body: { latitude: pending.lat, longitude: pending.lng }
      });
      setSaveMsg(t('maps.saved-to-listing'));
      setPending(null);
      await loadListings();
    } catch (err: any) {
      setSaveErr(err?.message || t('maps.save-fail'));
    } finally {
      setBusy(false);
    }
  };

  const saveAsNewListing = async () => {
    const name = newName.trim();
    if (!name) {
      setSaveErr(t('maps.name-required'));
      return;
    }
    if (!requireUsablePending() || !pending) return;
    setBusy(true);
    setSaveErr('');
    setSaveMsg('');
    try {
      const created = await api('/api/properties', {
        method: 'POST',
        body: {
          name,
          latitude: pending.lat,
          longitude: pending.lng,
          status: 'available'
        }
      });
      const id = created?.id as string | undefined;
      if (!id) throw new Error(t('maps.created-no-id'));
      const saved = { ...pending };
      setSaveMsg(t('maps.saved-private'));
      setPending(null);
      setNewName('');
      await loadListings();
      setSelectedId(id);
      setParams({ property: id }, { replace: true });
      if (isUsableMapPoint(saved.lat, saved.lng)) {
        mapRef.current?.easeTo({
          center: [saved.lng, saved.lat],
          zoom: Math.max(mapRef.current.getZoom(), 14)
        });
      } else {
        mapRef.current?.easeTo({ center: RIYADH_CENTER, zoom: 11 });
      }
    } catch (err: any) {
      setSaveErr(err?.message || t('maps.save-fail'));
    } finally {
      setBusy(false);
    }
  };

  const clearLocation = async () => {
    if (!selected) return;
    const ok = window.confirm(tf('maps.remove-confirm', { name: selected.name }));
    if (!ok) return;
    setBusy(true);
    setSaveErr('');
    setSaveMsg('');
    try {
      await api(`/api/properties/${selected.id}`, {
        method: 'PUT',
        body: { latitude: null, longitude: null }
      });
      setSaveMsg(t('maps.removed'));
      setPending(null);
      await loadListings();
    } catch (err: any) {
      setSaveErr(err?.message || t('maps.remove-fail'));
    } finally {
      setBusy(false);
    }
  };

  const startPlace = (id: string) => {
    selectListing(id);
    setPending(null);
    setSaveErr('');
    setSaveMsg('');
  };

  return (
    <div className="property-map-page">
      <div className="property-map-toolbar">
        <div>
          <h1 className="property-map-title"><MapPin size={18} strokeWidth={1.8} /> {t('maps.title')} <SectionGuideBanner id="maps" /></h1>
        </div>
        <div className="property-map-toolbar-actions">
          <button
            type="button"
            className="btn btn-outline btn-sm"
            onClick={() => {
              mapRef.current?.easeTo({ center: RIYADH_CENTER, zoom: 11 });
            }}
          >
            {t('maps.show-riyadh')}
          </button>
          <span className="property-map-count">{tf('maps.count', { on: located.length, off: unlocated.length })}</span>
        </div>
      </div>

      {loadError && <div className="error-alert">{loadError}</div>}
      {saveErr && <div className="error-alert">{saveErr}</div>}
      {saveMsg && <div className="success-alert">{saveMsg}</div>}

      <div className="property-map-body">
        <div className="property-map-canvas-wrap">
          <div ref={mapNode} className="property-map-canvas" />
          <div className="property-map-banner">
            {pending ? t('maps.banner-pending') : t('maps.banner-idle')}
          </div>
        </div>

        <aside className="property-map-side">
          <div className="property-map-editor">
            <ol className="property-map-steps">
              <li>{t('maps.step1')}</li>
              <li>{t('maps.step2')}</li>
              <li>{t('maps.step3')}</li>
            </ol>

            {pending ? (
              <div className="property-map-coords">
                {tf('maps.coords', { lat: pending.lat.toFixed(6), lng: pending.lng.toFixed(6) })}
              </div>
            ) : (
              <p className="property-map-hint">{t('maps.no-pending')}</p>
            )}

            <label className="property-map-field-label" htmlFor="map-place-name">{t('maps.place-name')}</label>
            <input
              id="map-place-name"
              className="form-input"
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
              placeholder={t('maps.place-ph')}
              disabled={busy}
            />
            <button
              type="button"
              className="btn btn-primary btn-sm"
              onClick={saveAsNewListing}
              disabled={!pending || busy}
            >
              <Plus size={14} strokeWidth={1.8} /> {t('maps.save-listing')}
            </button>

            {items.length > 0 && (
              <>
                <label className="property-map-field-label" htmlFor="map-existing-listing">{t('maps.link-existing')}</label>
                <select
                  id="map-existing-listing"
                  className="form-input select-input"
                  value={selectedId}
                  onChange={(e) => selectListing(e.target.value, true)}
                  disabled={busy}
                >
                  <option value="">{t('maps.choose-listing')}</option>
                  {items.map((item) => (
                    <option key={item.id} value={item.id}>
                      {item.name}{hasCoords(item) ? t('maps.has-location') : ''}
                    </option>
                  ))}
                </select>
                <button
                  type="button"
                  className="btn btn-primary btn-sm"
                  onClick={savePendingToListing}
                  disabled={!pending || !selected || busy}
                >
                  <Save size={14} strokeWidth={1.8} /> {t('maps.save-to-listing')}
                </button>
              </>
            )}
          </div>

          {selected && (
            <div className="property-map-editor">
              <div className="property-map-editor-name">{selected.name}</div>
              <div className="property-map-editor-meta">
                {[selected.city, selected.district, selected.property_type].filter(Boolean).join(' · ') || t('maps.no-city')}
              </div>
              <div className="property-map-editor-meta">
                {PROPERTY_STATUSES.find((s) => s.value === selected.status)?.label || selected.status || '—'}
                {selected.price != null ? ` · ${formatMoney(selected.price, selected.currency)}` : ''}
              </div>
              {hasCoords(selected) && !pending && (
                <div className="property-map-coords">
                  {Number(selected.latitude).toFixed(6)} ، {Number(selected.longitude).toFixed(6)}
                </div>
              )}
              {!hasCoords(selected) && (
                <p className="property-map-hint">{t('maps.invalid-city-point')}</p>
              )}
              <div className="property-map-editor-actions">
                <button type="button" className="btn btn-outline btn-sm" onClick={() => startPlace(selected.id)}>
                  <Crosshair size={14} strokeWidth={1.8} /> {hasCoords(selected) ? t('maps.move-pin') : t('maps.place-on-map')}
                </button>
                {hasCoords(selected) && (
                  <button type="button" className="btn btn-outline btn-sm" onClick={clearLocation} disabled={busy}>
                    <Trash2 size={14} strokeWidth={1.8} /> {t('maps.remove-from-map')}
                  </button>
                )}
                <button type="button" className="btn btn-outline btn-sm" onClick={() => navigate(`/properties/${selected.id}`)}>
                  <Pencil size={14} strokeWidth={1.8} /> {t('maps.edit-listing')}
                </button>
              </div>
            </div>
          )}

          <h3 className="property-map-list-title">
            <CircleAlert size={16} strokeWidth={1.8} aria-hidden />
            {t('maps.without-location')}
          </h3>
          <div className="property-map-list">
            {unlocated.length === 0 && (
              <div className="property-map-empty">
                {items.length === 0 ? t('maps.empty-no-listings') : t('maps.empty-all-located')}
              </div>
            )}
            {unlocated.map((item) => (
              <button
                key={item.id}
                type="button"
                className={`property-map-item ${selectedId === item.id ? 'is-selected' : ''}`}
                onClick={() => startPlace(item.id)}
              >
                <strong>{item.name}</strong>
                <span>{[item.city, item.district].filter(Boolean).join(' · ') || t('maps.click-to-locate')}</span>
              </button>
            ))}
          </div>

          <h3 className="property-map-list-title">
            <MapPinned size={16} strokeWidth={1.8} aria-hidden />
            {t('maps.on-map')}
          </h3>
          <div className="property-map-list">
            {located.length === 0 && <div className="property-map-empty">{t('maps.none-saved')}</div>}
            {located.map((item) => (
              <button
                key={item.id}
                type="button"
                className={`property-map-item ${selectedId === item.id ? 'is-selected' : ''}`}
                onClick={() => {
                  selectListing(item.id, true);
                }}
              >
                <strong>{item.name}</strong>
                <span>{[item.city, item.district].filter(Boolean).join(' · ') || t('maps.saved-for-you')}</span>
              </button>
            ))}
          </div>
        </aside>
      </div>
    </div>
  );
}
