import React, { useEffect, useMemo, useState } from 'react';
import { useSearchParams } from 'react-router-dom';
import { ChevronLeft, ChevronRight, CalendarDays, Pencil, Plus, Trash2, Clock } from 'lucide-react';
import { api } from '../api';
import BrandLogo from './BrandLogo';
import ObrClock from './ObrClock';
import { addMonths, isoDate, monthCells, monthLabel, parseIsoDate, WEEKDAY_LABELS } from '../schedule';
import PaginationBar, { usePagedItems } from '../components/PaginationBar';

const AGENDA_PAGE_SIZE = 4;

type Appointment = {
  id: string;
  title: string;
  occurs_on: string;
  starts_at?: string | null;
  notes?: string | null;
};

export default function OfficeAgenda() {
  const [searchParams, setSearchParams] = useSearchParams();
  const today = isoDate(new Date());
  const now = new Date();
  const [year, setYear] = useState(now.getFullYear());
  const [month, setMonth] = useState(now.getMonth());
  const [selected, setSelected] = useState(today);
  const [items, setItems] = useState<Appointment[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [formError, setFormError] = useState('');
  const [saving, setSaving] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [showForm, setShowForm] = useState(false);
  const [title, setTitle] = useState('');
  const [startsAt, setStartsAt] = useState('');
  const [notes, setNotes] = useState('');

  const cells = useMemo(() => monthCells(year, month), [year, month]);
  const range = useMemo(() => {
    const from = cells[0]?.iso || selected;
    const to = cells[cells.length - 1]?.iso || selected;
    return { from, to };
  }, [cells, selected]);

  const load = async () => {
    setLoading(true);
    setError('');
    try {
      const data = await api(`/api/appointments?from=${range.from}&to=${range.to}`);
      setItems(Array.isArray(data) ? data : []);
    } catch (err: any) {
      setError(err?.message || 'تعذر تحميل المواعيد.');
      setItems([]);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    load();
  }, [range.from, range.to]);

  const daysWithAppointments = useMemo(() => {
    const set = new Set<string>();
    for (const item of items) {
      if (item.occurs_on) set.add(item.occurs_on);
    }
    return set;
  }, [items]);

  const dayItems = items
    .filter((item) => item.occurs_on === selected)
    .sort((a, b) => (a.starts_at || '99:99').localeCompare(b.starts_at || '99:99'));

  const pagedDayItems = usePagedItems(dayItems, AGENDA_PAGE_SIZE, selected);

  const goToday = () => {
    const d = new Date();
    setYear(d.getFullYear());
    setMonth(d.getMonth());
    setSelected(isoDate(d));
  };

  const moveMonth = (delta: number) => {
    const next = addMonths(year, month, delta);
    setYear(next.year);
    setMonth(next.month);
  };

  const openCreate = () => {
    setEditingId(null);
    setTitle('');
    setStartsAt('');
    setNotes('');
    setFormError('');
    setShowForm(true);
  };

  useEffect(() => {
    if (searchParams.get('appointment') !== 'new') return;
    openCreate();
    const next = new URLSearchParams(searchParams);
    next.delete('appointment');
    setSearchParams(next, { replace: true });
    window.requestAnimationFrame(() => {
      document.querySelector('.office-agenda')?.scrollIntoView({ behavior: 'smooth', block: 'start' });
    });
  }, [searchParams, setSearchParams]);

  const openEdit = (item: Appointment) => {
    setEditingId(item.id);
    setTitle(item.title || '');
    setStartsAt(item.starts_at || '');
    setNotes(item.notes || '');
    setFormError('');
    setShowForm(true);
  };

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault();
    setFormError('');
    setSaving(true);
    try {
      const body = {
        title: title.trim(),
        occurs_on: selected,
        starts_at: startsAt,
        notes: notes.trim()
      };
      if (editingId) {
        await api(`/api/appointments/${editingId}`, { method: 'PUT', body });
      } else {
        await api('/api/appointments', { method: 'POST', body });
      }
      setShowForm(false);
      setEditingId(null);
      await load();
    } catch (err: any) {
      setFormError(err?.message || 'تعذر حفظ الموعد.');
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = async (item: Appointment) => {
    if (!window.confirm(`حذف موعد «${item.title}»؟`)) return;
    setError('');
    try {
      await api(`/api/appointments/${item.id}`, { method: 'DELETE' });
      await load();
    } catch (err: any) {
      setError(err?.message || 'تعذر حذف الموعد.');
    }
  };

  const selectedLabel = (() => {
    const d = parseIsoDate(selected);
    return `${d.getDate()} ${monthLabel(d.getFullYear(), d.getMonth())}`;
  })();

  return (
    <section className="office-agenda">
      <header className="office-agenda-head">
        <div className="office-agenda-brand">
          <BrandLogo className="obr-calendar-logo" />
          <div>
            <div className="office-agenda-kicker">
              <Clock size={14} strokeWidth={1.8} aria-hidden />
              الوقت والمواعيد
            </div>
            <div className="office-agenda-hint">الساعة والتقويم حسب جهازك — المواعيد محفوظة محليًا</div>
          </div>
        </div>
        <div className="obr-calendar-nav" dir="ltr">
          <button type="button" className="obr-calendar-arrow" onClick={() => moveMonth(-1)} aria-label="الشهر السابق">
            <ChevronLeft size={16} />
          </button>
          <div className="obr-calendar-month" dir="rtl">{monthLabel(year, month)}</div>
          <button type="button" className="obr-calendar-arrow" onClick={() => moveMonth(1)} aria-label="الشهر التالي">
            <ChevronRight size={16} />
          </button>
        </div>
      </header>

      <div className="office-agenda-body">
        <div className="office-agenda-pane is-clock">
          <ObrClock showBrand={false} />
        </div>

        <div className="office-agenda-pane is-calendar">
          <div className="obr-calendar-grid" dir="ltr">
            {WEEKDAY_LABELS.map((label) => (
              <div key={label} className="obr-calendar-dow">{label}</div>
            ))}
            {cells.map((cell) => {
              const isToday = cell.iso === today;
              const isSelected = cell.iso === selected;
              return (
                <button
                  key={cell.iso}
                  type="button"
                  className={`obr-calendar-day${cell.inMonth ? '' : ' is-out'}${isToday ? ' is-today' : ''}${isSelected ? ' is-selected' : ''}`}
                  onClick={() => setSelected(cell.iso)}
                >
                  <span>{cell.day}</span>
                  {daysWithAppointments.has(cell.iso) && <i className="obr-calendar-dot" aria-hidden="true" />}
                </button>
              );
            })}
          </div>
          <button type="button" className="obr-calendar-today" onClick={goToday}>
            <CalendarDays size={15} strokeWidth={1.8} /> اليوم
          </button>
        </div>

        <div className="office-agenda-pane is-appointments">
          <div className="obr-appointments-head">
            <div>
              <div className="obr-appointments-kicker">
                <CalendarDays size={14} strokeWidth={1.8} aria-hidden />
                مواعيد اليوم المحدد
              </div>
              <div className="obr-appointments-title">{selectedLabel}</div>
            </div>
            <button type="button" className="btn btn-primary" onClick={openCreate}>
              <Plus size={14} strokeWidth={1.8} /> موعد
            </button>
          </div>

          {error && <div className="error-alert">{error}</div>}

          {showForm && (
            <form className="obr-appointment-form" onSubmit={handleSave}>
              {formError && <div className="error-alert">{formError}</div>}
              <div className="form-group">
                <label htmlFor="appt-title">عنوان الموعد</label>
                <input id="appt-title" className="form-input" value={title} required minLength={2} onChange={(e) => setTitle(e.target.value)} />
              </div>
              <div className="form-group">
                <label htmlFor="appt-time">الوقت</label>
                <input id="appt-time" className="form-input" type="time" value={startsAt} onChange={(e) => setStartsAt(e.target.value)} />
              </div>
              <div className="form-group">
                <label htmlFor="appt-notes">ملاحظات</label>
                <input id="appt-notes" className="form-input" value={notes} onChange={(e) => setNotes(e.target.value)} />
              </div>
              <div style={{ display: 'flex', gap: 8 }}>
                <button className="btn btn-primary" type="submit" disabled={saving}>{saving ? 'جاري الحفظ...' : 'حفظ'}</button>
                <button className="btn btn-outline" type="button" onClick={() => setShowForm(false)}>إلغاء</button>
              </div>
            </form>
          )}

          {loading ? (
            <div>جاري تحميل المواعيد...</div>
          ) : dayItems.length === 0 ? (
            <div className="obr-appointments-empty">لا مواعيد في هذا اليوم. أضف موعدًا يدويًا.</div>
          ) : (
            <>
            <ul className="obr-appointment-list">
              {pagedDayItems.slice.map((item) => (
                <li key={item.id} className="obr-appointment-item">
                  <div>
                    <div className="obr-appointment-time num">{item.starts_at || 'بدون وقت'}</div>
                    <div className="obr-appointment-name">{item.title}</div>
                    {item.notes && <div className="obr-appointment-notes">{item.notes}</div>}
                  </div>
                  <div className="campaign-card-actions">
                    <button className="btn btn-outline" type="button" onClick={() => openEdit(item)} aria-label={`تعديل ${item.title}`}>
                      <Pencil size={14} />
                    </button>
                    <button className="btn btn-outline" type="button" onClick={() => handleDelete(item)} aria-label={`حذف ${item.title}`}>
                      <Trash2 size={14} />
                    </button>
                  </div>
                </li>
              ))}
            </ul>
            <PaginationBar total={pagedDayItems.total} page={pagedDayItems.page} onPage={pagedDayItems.setPage} pageSize={AGENDA_PAGE_SIZE} label="موعد" />
            </>
          )}
        </div>
      </div>
    </section>
  );
}
