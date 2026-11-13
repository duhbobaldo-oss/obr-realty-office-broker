import { integrationRequest } from './OfficialIntegrations';
import { t } from '../domain/locale';
import { getCountry, countryProfile } from '../domain/countryProfile';
import React, { useEffect, useMemo, useState } from 'react';
import { ExternalLink, Eye, FileUp, FileText, Plus, Printer, Save, Trash2, X } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { api } from '../api';
import {
  BROKERAGE_PORTAL_BUTTON_LABEL,
  DRAFT_UNVERIFIED_BADGE,
  isUnofficialContract
} from '../contracts';

const blank = () => ({ country: getCountry(), currency: countryProfile(getCountry()).currency, contract_number: '', contract_type: '', contract_scope: '', parties: [], property_id: '', client_id: '', campaign_id: '', commission_id: '', invoice_id: '', start_date: '', end_date: '', status: 'draft', commission_amount: '', commission_mechanism: '', license_number: '', source: 'manual_draft', provider_fields_json: {} as Record<string, unknown> });

const STATUS_LABEL: Record<string, string> = { draft: 'مسودة', unverified: 'غير موثق', documented: 'موثق', active: 'نشط', expired: 'منتهي', cancelled: 'ملغى' };
const COMMISSION_LABEL: Record<string, string> = { percent: 'نسبة مئوية', fixed: 'مبلغ ثابت' };

function escHtml(s: unknown): string {
  return String(s ?? '').replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
}

export default function ContractsTab({ newDraftSignal = 0 }: { newDraftSignal?: number }) {
  const [items, setItems] = useState<any[]>([]); const [summary, setSummary] = useState<any>({});
  const [clients, setClients] = useState<any[]>([]); const [properties, setProperties] = useState<any[]>([]); const [campaigns, setCampaigns] = useState<any[]>([]); const [commissions, setCommissions] = useState<any[]>([]); const [invoices, setInvoices] = useState<any[]>([]);
  const [form, setForm] = useState<any | null>(null); const [saving, setSaving] = useState(false); const [error, setError] = useState(''); const [note, setNote] = useState('');
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [draftPreview, setDraftPreview] = useState<any | null>(null);
  const [pdfPreview, setPdfPreview] = useState<{ title: string; name: string; dataUrl: string } | null>(null);
  const [portalAssist, setPortalAssist] = useState<{ contract: any; portalOpened: boolean; result: any | null } | null>(null);
  const [pdfLoading, setPdfLoading] = useState(false);
  const load = async () => { const [c,s,cl,p,ca,co,i] = await Promise.all([api('/api/contracts'),api('/api/contracts/summary'),api('/api/clients?limit=200'),api('/api/properties?limit=200'),api('/api/campaigns?limit=200'),api('/api/finance/commissions'),api('/api/finance/invoices')]); setItems(c.items || []); setSummary(s); setClients(cl.items || []); setProperties(p.items || []); setCampaigns(ca.items || []); setCommissions(co.items || []); setInvoices(i.items || []); };
  useEffect(() => { load().catch((e) => setError(e?.message || 'تعذر تحميل العقود')); }, []);
  useEffect(() => {
    if (!draftPreview && !pdfPreview && !portalAssist) return;
    const previousOverflow = document.body.style.overflow;
    document.body.style.overflow = 'hidden';
    return () => { document.body.style.overflow = previousOverflow; };
  }, [draftPreview, pdfPreview]);
  // إشارة من زر + في المالية: فتح نموذج مسودة جديدة مباشرة.
  const firstSignal = React.useRef(true);
  useEffect(() => {
    if (firstSignal.current) { firstSignal.current = false; return; }
    if (newDraftSignal > 0) { setError(''); setNote(''); setForm(blank()); }
  }, [newDraftSignal]);
  // رابط جهة التوثيق Configurable: من الخادم أولًا — لا يوجد أي رابط ثابت داخل الواجهة.
  const openPortal = async () => {
    try {
      const record = portalAssist?.contract || draftPreview || items.find(x => x.id === selectedId);
      const service = await integrationRequest({ action: 'resolve', country: record?.country, licenseType: record?.broker_license_type });
      if (service.mode === 'api') { setNote(t(service.status === 'success' ? 'integration.connectedMapping' : 'integration.api_ready')); return; }
      if (!service.url) { setError(t('integration.no_service')); return; }
      await invoke('open_external_url', { url: service.url });
    } catch { setError(t('integration.failed')); }
  };
  const nameOf = (list: any[], id: string) => list.find((x) => x.id === id)?.name || list.find((x) => x.id === id)?.title || list.find((x) => x.id === id)?.number || '';
  const selected = items.find((x) => x.id === selectedId) || null;

  const choosePdf = async (e: React.ChangeEvent<HTMLInputElement>) => { const file=e.target.files?.[0]; if(!file) return; if(file.type !== 'application/pdf' && !file.name.toLowerCase().endsWith('.pdf')) { setError('يُقبل ملف PDF فقط.'); return; } const data=await new Promise<string>((resolve,reject)=>{const r=new FileReader();r.onload=()=>resolve(String(r.result));r.onerror=reject;r.readAsDataURL(file);}); try { const preview:any=await api('/api/contracts/preview-pdf',{method:'POST',body:{data_url:data}}); setForm({ ...blank(), ...(preview.fields||{}), source:'official_pdf', status:'unverified', pdf_name:file.name, pdf_data_url:data, extracted_text_excerpt:preview.extracted_text_excerpt||'' }); } catch(err:any) { setError(err?.message||'تعذر قراءة ملف PDF'); } };
  const save = async (e: React.FormEvent) => { e.preventDefault(); if (!form) return; const rawCommission = String(form.commission_amount ?? '').trim(); const commissionAmount = rawCommission ? Number(rawCommission.replace(/[٪%،,\s]/g, '')) : null; if (rawCommission && !Number.isFinite(commissionAmount)) { setError('أدخل قيمة عمولة رقمية، ويمكن كتابة % عند اختيار نسبة مئوية.'); return; } setSaving(true); setError(''); try { const res:any = await api('/api/contracts', { method:'POST', body: { ...form, commission_amount: commissionAmount } }); setForm(null); await load(); if (res?.id) setSelectedId(res.id); } catch(e:any) { setError(e?.message || 'تعذر حفظ العقد'); } finally { setSaving(false); } };
  const deleteContract = async (id: string) => {
    if (!window.confirm('حذف هذا العقد وملف PDF المرتبط به نهائيًا؟')) return;
    try {
      await api(`/api/contracts/${id}`, { method: 'DELETE' });
      setDraftPreview(null);
      setSelectedId(null);
      await load();
    } catch (e: any) {
      setError(e?.message || 'تعذر حذف العقد');
    }
  };
  const set=(key:string,value:any)=>setForm((f:any)=>{
    const next = {...f,[key]:value};
    if (!next.id) {
      const parent = invoices.find(x=>x.id===next.invoice_id) || commissions.find(x=>x.id===next.commission_id)
        || properties.find(x=>x.id===next.property_id) || campaigns.find(x=>x.id===next.campaign_id);
      next.country = parent?.country || getCountry();
      next.currency = parent?.currency || countryProfile(next.country).currency;
      next.broker_license_type = parent?.broker_license_type;
    }
    return next;
  });

  /** طباعة المسودة: توليد PDF خلفي بنفس مسار فواتير المالية وفتحه في عارض النظام (للطباعة منه).
      طباعة WebView المباشرة غير موثوقة في Tauri — عند تعذر الخادم يُستخدم iframe كاحتياط. */
  const printDraft = async (c: any) => {
    setError(''); setNote(''); setPdfLoading(true);
    const payload = {
      ...c,
      parties: Array.isArray(c.parties) ? c.parties : (c.parties ? String(c.parties).split(/[،,]/).map((x: string) => x.trim()).filter(Boolean) : []),
      property_name: c.property_name || nameOf(properties, c.property_id) || '',
      client_name: c.client_name || nameOf(clients, c.client_id) || ''
    };
    try {
      const res: any = await api('/api/contracts/print-pdf', { method: 'POST', body: { ...payload, open: true } });
      setNote(`تم تجهيز PDF الطباعة${res?.pdf_path ? `: ${res.pdf_path}` : ''} — أكمل الطباعة من عارض النظام.`);
      setPdfLoading(false);
      return;
    } catch (err: any) {
      setError(err?.message || 'تعذر تجهيز PDF — جارٍ استخدام الطباعة البديلة.');
      setPdfLoading(false);
    }
    printDraftFallback(c).catch(() => {});
  };

  const printDraftFallback = async (c: any) => {
    // هوية المكتب من الإعدادات فقط — خانة فارغة = إخفاء العنوان، لا أي Branding.
    let officeName = '';
    try {
      const s: any = await api('/api/settings');
      officeName = String(s?.office_name || '').trim();
    } catch {
      officeName = '';
    }
    const rows: [string, string][] = [
      ['النوع', c.contract_type || '—'],
      ['نطاق العقد', c.contract_scope || '—'],
      ['رقم العقد', c.contract_number || '—'],
      ['الأطراف', Array.isArray(c.parties) ? (c.parties.join(' · ') || '—') : (c.parties || '—')],
      ['العقار', c.property_name || nameOf(properties, c.property_id) || '—'],
      ['العميل', c.client_name || nameOf(clients, c.client_id) || '—'],
      ['المدة', [c.start_date, c.end_date].filter(Boolean).join(' ← ') || '—'],
      ['تاريخ البداية', c.start_date || '—'],
      ['تاريخ النهاية', c.end_date || '—'],
      ['العمولة', [c.commission_amount ?? '—', c.commission_mechanism === 'percent' ? '%' : c.currency, COMMISSION_LABEL[c.commission_mechanism] || c.commission_mechanism || ''].filter(Boolean).join(' ') || '—'],
      ['الحالة', STATUS_LABEL[String(c.status || '')] || String(c.status || '—')],
      ['رقم الترخيص / السجل التجاري', c.license_number || '—'],
      ['المصدر', c.source === 'official_pdf' ? 'PDF موثق' : 'مسودة يدوية']
    ];
    const badge = isUnofficialContract(c) ? `<div style="display:inline-block;background:#d97706;color:#fff;font-weight:700;padding:4px 14px;border-radius:999px;margin:8px 0;">${DRAFT_UNVERIFIED_BADGE}</div>` : '';
    const heading = officeName ? `<h1 style="margin:0;font-size:22px;">${escHtml(officeName)}</h1>` : '';
    const html = `<!DOCTYPE html><html dir="rtl" lang="ar"><head><meta charset="utf-8"><title>عقد ${escHtml(c.contract_number || '')}</title></head><body style="font-family:Tahoma,Arial,sans-serif;background:#fff;color:#23365f;margin:0;padding:32px;">`
      + `<div style="border-top:6px solid #23365f;border-bottom:3px solid #d97706;padding:12px 0;margin-bottom:16px;">${heading}${badge}</div>`
      + `<table style="width:100%;border-collapse:collapse;">${rows.map(([k,v]) => `<tr><th style="text-align:right;background:#f7f1e8;border:1px solid #e3dcc9;padding:8px;width:32%;">${escHtml(k)}</th><td style="border:1px solid #e3dcc9;padding:8px;">${escHtml(v)}</td></tr>`).join('')}</table>`
      + `</body></html>`;
    const frame = document.createElement('iframe');
    frame.style.position = 'fixed'; frame.style.width = '0'; frame.style.height = '0'; frame.style.border = '0';
    document.body.appendChild(frame);
    const doc = frame.contentDocument || frame.contentWindow?.document;
    if (!doc) { frame.remove(); setError('تعذر فتح نافذة الطباعة.'); return; }
    doc.open(); doc.write(html); doc.close();
    frame.onload = () => { try { frame.contentWindow?.focus(); frame.contentWindow?.print(); } catch { /* تجاهل */ } setTimeout(() => frame.remove(), 1500); };
  };

  /** معاينة PDF المستورد داخل التطبيق — الملف الأصلي يُقرأ فقط دون أي تعديل. */
  const previewImportedPdf = async (c: any) => {
    if (c.pdf_data_url) { setPdfPreview({ title: `معاينة PDF — ${c.contract_number || c.pdf_name || ''}`, name: c.pdf_name || 'contract.pdf', dataUrl: c.pdf_data_url }); return; }
    if (!c.id) return;
    setPdfLoading(true); setError('');
    try {
      const res: any = await api(`/api/contracts/${c.id}/pdf-data`);
      setPdfPreview({ title: `معاينة PDF — ${c.contract_number || res.pdf_name || ''}`, name: res.pdf_name || 'contract.pdf', dataUrl: res.data_url });
    } catch (err: any) { setError(err?.message || 'تعذر فتح معاينة PDF'); }
    finally { setPdfLoading(false); }
  };

  /** فتح الملف الأصلي في عارض النظام (يتضمن الطباعة من العارض). */
  const openImportedPdf = async (c: any) => {
    if (c.pdf_data_url) {
      const w = window.open('', '_blank');
      if (!w) { setError('تعذر فتح نافذة جديدة.'); return; }
      w.document.write(`<html dir="rtl"><body style="margin:0"><embed src="${c.pdf_data_url}" type="application/pdf" style="width:100vw;height:100vh"></body></html>`);
      return;
    }
    if (!c.id) return;
    setPdfLoading(true); setError('');
    try { await api(`/api/contracts/${c.id}/open-pdf`, { method: 'POST', body: {} }); }
    catch (err: any) { setError(err?.message || 'تعذر فتح ملف PDF'); }
    finally { setPdfLoading(false); }
  };

  const hasPdf = (c: any) => Boolean(c?.pdf_data_url || c?.pdf_asset_id || c?.pdf_name || c?.pdf_path);
  const portalContract = (c: any) => ({
    ...c,
    parties: Array.isArray(c.parties) ? c.parties : String(c.parties || '').split(/[،,]/).map((value) => value.trim()).filter(Boolean),
    property_name: c.property_name || nameOf(properties, c.property_id) || '',
  });
  const startPortalAssist = (c: any) => setPortalAssist({ contract: portalContract(c), portalOpened: false, result: null });
  const preparePortalAssist = async () => {
    if (!portalAssist?.portalOpened) return;
    try {
      setError('');
      const result: any = await api('/api/contracts/portal-assist', { method: 'POST', body: portalAssist.contract });
      setPortalAssist((current) => current ? { ...current, result } : current);
    } catch (e: any) {
      setError(e?.message || 'تعذر تجهيز بيانات البوابة.');
    }
  };
  const copyPortalValue = async (value: string) => {
    try { await navigator.clipboard.writeText(value); setNote('تم نسخ قيمة الحقل.'); }
    catch { setError('تعذر النسخ التلقائي؛ انسخ القيمة يدويًا.'); }
  };
  const portalReviewRows = portalAssist ? [
    ['النوع', portalAssist.contract.contract_type || '—'],
    ['الأطراف', Array.isArray(portalAssist.contract.parties) ? (portalAssist.contract.parties.join(' · ') || '—') : '—'],
    ['العقار', portalAssist.contract.property_name || '—'],
    ['المدة', [portalAssist.contract.start_date, portalAssist.contract.end_date].filter(Boolean).join(' ← ') || '—'],
    ['العمولة', [portalAssist.contract.commission_amount || '', COMMISSION_LABEL[portalAssist.contract.commission_mechanism] || ''].filter(Boolean).join(' ') || '—'],
    ['نطاق العقد', portalAssist.contract.contract_scope || '—'],
  ] : [];

  return <div className="contracts-workspace">
    <div className="contracts-head"><div><h3>العقود</h3><p className="contracts-head-desc">سجل داخلي للعقود؛ إجراءات الجهة الرسمية تتطلب بوابتها أو ربط حقول موثقًا وفق API المعتمد.</p></div><div className="contracts-actions"><button className="btn btn-primary" type="button" onClick={()=>setForm(blank())}><Plus size={15} strokeWidth={1.8}/> + عقد جديد</button><button className="btn btn-outline" type="button" onClick={()=>document.getElementById('contract-pdf')?.click()}><FileUp size={15} strokeWidth={1.8}/> استيراد PDF موثق</button><input id="contract-pdf" hidden type="file" accept="application/pdf,.pdf" onChange={choosePdf}/><button className="btn btn-outline" type="button" onClick={openPortal}><ExternalLink size={15} strokeWidth={1.8}/> {BROKERAGE_PORTAL_BUTTON_LABEL}</button></div></div>
    {error && <div className="error-alert">{error}</div>}
    {note && <div className="alert-note">{note}</div>}
    {form && <form className="content-card obr-panel contracts-form" onSubmit={save}><div className="contracts-form-header"><div className="card-title"><FileText size={17} strokeWidth={1.8}/> {form.source==='official_pdf'?'مراجعة العقد المستورد قبل الحفظ':'إضافة مسودة عقد'}</div></div>{form.source==='official_pdf' && <div className="alert-note">تم حفظ الملف الأصلي عند الحفظ. استُخرجت فقط الحقول ذات الدلالة الصريحة من PDF النصي؛ الحقول غير المؤكدة بقيت فارغة للمراجعة.</div>}{form.source==='official_pdf' && (form.extracted_text_excerpt ? <div className="muted contracts-excerpt">معاينة نصية: {form.extracted_text_excerpt}</div> : <div className="muted contracts-excerpt">تعذر استخراج معاينة نصية موثوقة من ملف PDF.</div>)}<div className="contract-form-layout">
      <section className="contract-form-section"><div className="contract-form-section-title"><span>01</span><div><strong>بيانات العقد</strong><small>عرّف العقد ونطاقه وأطرافه.</small></div></div><div className="form-grid"><Field label="رقم العقد"><input className="form-input" value={form.contract_number} onChange={e=>set('contract_number',e.target.value)} placeholder="مثال: 123456789"/></Field><Field label="نوع العقد"><input className="form-input" value={form.contract_type} onChange={e=>set('contract_type',e.target.value)} placeholder="مثال: وساطة بيع / تأجير / تسويق"/></Field><Field label="نطاق العقد"><input className="form-input" value={form.contract_scope} onChange={e=>set('contract_scope',e.target.value)} placeholder="مثال: تسويق العقار / البحث عن مشتري"/></Field><Field label="الأطراف"><input className="form-input" value={(form.parties||[]).join('، ')} onChange={e=>set('parties',e.target.value.split('،').map(x=>x.trim()).filter(Boolean))} placeholder="مثال: المالك، الوسيط، المشتري"/></Field></div></section>
      <section className="contract-form-section"><div className="contract-form-section-title"><span>02</span><div><strong>الروابط</strong><small>اربط العقد بالسجلات الموجودة عند الحاجة.</small></div></div><div className="form-grid"><Select label="العقار المرتبط" value={form.property_id} onChange={v=>set('property_id',v)} items={properties}/><Select label="العميل المرتبط" value={form.client_id} onChange={v=>set('client_id',v)} items={clients}/><Select label="الحملة المرتبطة" value={form.campaign_id} onChange={v=>set('campaign_id',v)} items={campaigns}/><Select label="العمولة المرتبطة" value={form.commission_id} onChange={v=>set('commission_id',v)} items={commissions}/><Select label="الفاتورة المرتبطة" value={form.invoice_id} onChange={v=>set('invoice_id',v)} items={invoices}/></div></section>
      <section className="contract-form-section"><div className="contract-form-section-title"><span>03</span><div><strong>المدة والتوثيق</strong><small>حدّد الحالة والعمولة والمرجع النظامي.</small></div></div><div className="form-grid"><Field label="تاريخ البداية"><input className="form-input" type="date" value={form.start_date} onChange={e=>set('start_date',e.target.value)}/></Field><Field label="تاريخ النهاية"><input className="form-input" type="date" value={form.end_date} onChange={e=>set('end_date',e.target.value)}/></Field><Field label="الحالة"><select className="form-input" value={form.status} onChange={e=>set('status',e.target.value)}><option value="draft">مسودة / غير موثق</option><option value="documented">موثق</option><option value="active">نشط</option><option value="expired">منتهي</option><option value="cancelled">ملغى</option></select></Field><Field label="قيمة العمولة"><input className="form-input" inputMode="decimal" value={form.commission_amount} onChange={e=>set('commission_amount',e.target.value)} placeholder={form.commission_mechanism==='percent' ? 'مثال: 2.5%' : 'مثال: 10000'}/></Field><Field label="آلية العمولة"><select className="form-input" value={form.commission_mechanism} onChange={e=>set('commission_mechanism',e.target.value)}><option value="">اختر آلية العمولة</option><option value="percent">نسبة مئوية</option><option value="fixed">مبلغ ثابت</option></select></Field><Field label="رقم الترخيص / السجل التجاري"><input className="form-input" value={form.license_number} onChange={e=>set('license_number',e.target.value)} placeholder="رقم الترخيص / السجل التجاري"/></Field></div></section>
    </div><div className="contracts-file">{form.pdf_name ? <>PDF: <b>{form.pdf_name}</b>{form.pdf_data_url && <button className="btn btn-outline btn-sm" type="button" onClick={()=>setPdfPreview({ title: `معاينة PDF — ${form.pdf_name}`, name: form.pdf_name, dataUrl: form.pdf_data_url })}><Eye size={14} strokeWidth={1.8}/> معاينة PDF</button>}</> : 'لا يوجد PDF مرفق في المسودة.'}</div><div className="contracts-actions"><button className="btn btn-outline" type="button" onClick={()=>setDraftPreview(form.source==='official_pdf' ? {...form, property_name: nameOf(properties, form.property_id), client_name: nameOf(clients, form.client_id)} : form)}><Eye size={15} strokeWidth={1.8}/> معاينة</button><button className="btn btn-outline" type="button" disabled={pdfLoading} onClick={()=>printDraft({...form, property_name: nameOf(properties, form.property_id), client_name: nameOf(clients, form.client_id)})}><Printer size={15} strokeWidth={1.8}/> طباعة</button><button className="btn btn-outline" type="button" onClick={()=>startPortalAssist(form)}><FileText size={15} strokeWidth={1.8}/> تجهيز بيانات العقد</button><button className="btn btn-primary" disabled={saving}><Save size={15} strokeWidth={1.8}/>{saving?'جارٍ الحفظ…':'حفظ العقد'}</button>{form.source==='manual_draft'&&<button className="btn btn-outline" type="button" onClick={openPortal}><ExternalLink size={15} strokeWidth={1.8}/> {BROKERAGE_PORTAL_BUTTON_LABEL}</button>}<button className="btn btn-outline" type="button" onClick={()=>setForm(null)}>إلغاء</button></div></form>}

    {selected && <div className="content-card obr-panel contracts-details"><div className="card-header"><div className="card-title"><FileText size={17} strokeWidth={1.8}/> تفاصيل العقد — {selected.contract_number || 'بدون رقم'}{isUnofficialContract(selected) && <span className="finance-status finance-status-draft"> {DRAFT_UNVERIFIED_BADGE}</span>}</div><button className="btn btn-outline btn-sm" type="button" onClick={()=>setSelectedId(null)}><X size={14} strokeWidth={1.8}/> إغلاق</button></div><ContractFields c={selected} /><div className="contracts-actions"><button className="btn btn-outline" type="button" onClick={()=>setDraftPreview(selected)}><Eye size={15} strokeWidth={1.8}/> معاينة</button><button className="btn btn-outline" type="button" disabled={pdfLoading} onClick={()=>printDraft(selected)}><Printer size={15} strokeWidth={1.8}/> طباعة</button><button className="btn btn-outline" type="button" onClick={()=>startPortalAssist(selected)}><FileText size={15} strokeWidth={1.8}/> تجهيز بيانات العقد</button>{hasPdf(selected) && <><button className="btn btn-outline" type="button" onClick={()=>previewImportedPdf(selected)} disabled={pdfLoading}><Eye size={15} strokeWidth={1.8}/> معاينة PDF</button><button className="btn btn-outline" type="button" onClick={()=>openImportedPdf(selected)} disabled={pdfLoading}><ExternalLink size={15} strokeWidth={1.8}/> فتح PDF</button></>}<button className="btn btn-outline" type="button" onClick={openPortal}><ExternalLink size={15} strokeWidth={1.8}/> {BROKERAGE_PORTAL_BUTTON_LABEL}</button></div></div>}

    <div className="content-card obr-panel"><div className="table-container"><table className="data-table"><thead><tr><th>رقم العقد</th><th>النوع / الأطراف</th><th>العقار</th><th>المدة</th><th>الحالة</th><th>العمولة</th><th>المصدر / PDF</th><th>إجراءات</th></tr></thead><tbody>{!items.length?<tr><td colSpan={8}>لا توجد عقود بعد.</td></tr>:items.map(x=><tr key={x.id} onClick={()=>setSelectedId(x.id)} style={{cursor:'pointer'}}><td>{x.contract_number||'—'}</td><td>{[x.contract_type,x.contract_scope].filter(Boolean).join(' · ')||'—'}<div className="muted">{(x.parties||[]).join(' · ')||'—'}</div></td><td>{x.property_name||'—'}{x.client_name&&<div className="muted">{x.client_name}</div>}</td><td>{[x.start_date,x.end_date].filter(Boolean).join(' ← ')||'—'}</td><td><span className={`finance-status finance-status-${x.status}`}>{STATUS_LABEL[x.status]||x.status}</span>{isUnofficialContract(x)&&<div className="muted">{DRAFT_UNVERIFIED_BADGE}</div>}</td><td>{x.commission_amount??'—'} {x.commission_mechanism === 'percent' ? '%' : x.currency}</td><td>{x.source==='official_pdf'?'PDF موثق':'مسودة'}<div className="muted">{x.pdf_name||'بدون PDF'}</div></td><td onClick={(e)=>e.stopPropagation()}><div className="contracts-actions"><button className="btn btn-outline btn-sm" type="button" onClick={()=>setDraftPreview(x)}><Eye size={14} strokeWidth={1.8}/> معاينة</button><button className="btn btn-outline btn-sm" type="button" disabled={pdfLoading} onClick={()=>printDraft(x)}><Printer size={14} strokeWidth={1.8}/> طباعة</button><button className="btn btn-outline btn-sm contract-delete-btn" type="button" onClick={()=>deleteContract(x.id)}><Trash2 size={14} strokeWidth={1.8}/> حذف</button>{hasPdf(x)&&<button className="btn btn-outline btn-sm" type="button" onClick={()=>openImportedPdf(x)} disabled={pdfLoading}><ExternalLink size={14} strokeWidth={1.8}/> فتح PDF</button>}</div></td></tr>)}</tbody></table></div></div>

    {draftPreview && <div className="contracts-modal" role="dialog" aria-modal="true" onClick={()=>setDraftPreview(null)}><div className="content-card obr-panel contracts-modal-card" onClick={(e)=>e.stopPropagation()}><div className="contracts-modal-header"><div className="card-title"><FileText size={17} strokeWidth={1.8}/> معاينة العقد — {draftPreview.contract_number || 'بدون رقم'}{isUnofficialContract(draftPreview) && <span className="finance-status finance-status-draft"> {DRAFT_UNVERIFIED_BADGE}</span>}</div><div className="contracts-modal-actions"><button className="btn btn-primary btn-sm" type="button" disabled={!form || saving} onClick={()=>save({ preventDefault: ()=>{} } as React.FormEvent)}><Save size={14} strokeWidth={1.8}/> حفظ العقد</button><button className="btn btn-outline btn-sm" type="button" onClick={()=>startPortalAssist(draftPreview)}><FileText size={14} strokeWidth={1.8}/> تعبئة من OBR</button><button className="btn btn-outline btn-sm" type="button" onClick={openPortal}><ExternalLink size={14} strokeWidth={1.8}/> الخدمات الرسمية للوساطة العقارية</button><button className="btn btn-outline btn-sm" type="button" disabled={pdfLoading} onClick={()=>printDraft(draftPreview)}><Printer size={14} strokeWidth={1.8}/> طباعة</button>{draftPreview.id && <button className="btn btn-outline btn-sm contract-delete-btn" type="button" onClick={()=>deleteContract(draftPreview.id)}><Trash2 size={14} strokeWidth={1.8}/> حذف</button>}<button className="btn btn-outline btn-sm" type="button" onClick={()=>setDraftPreview(null)}><X size={14} strokeWidth={1.8}/> إغلاق</button></div></div><div className="contracts-modal-body"><ContractFields c={draftPreview} /></div></div></div>}

    {portalAssist && <div className="contracts-modal" role="dialog" aria-modal="true" aria-label="مساعد تجهيز بيانات العقد" onClick={()=>setPortalAssist(null)}><div className="content-card obr-panel contracts-modal-card portal-assist-modal" onClick={(e)=>e.stopPropagation()}><div className="contracts-modal-header"><div className="card-title"><FileText size={17} strokeWidth={1.8}/> تجهيز بيانات العقد</div><button className="btn btn-outline btn-sm" type="button" onClick={()=>setPortalAssist(null)}><X size={14} strokeWidth={1.8}/> إغلاق</button></div><div className="contracts-modal-body"><p className="portal-assist-note">راجع البيانات أولًا. سجّل دخولك إلى الخدمات الرسمية بنفسك؛ لا يخزّن OBR بيانات الدخول ولا يكتب داخل البوابة أو يتجاوز CAPTCHA/MFA.</p><div className="portal-assist-review">{portalReviewRows.map(([label,value])=><div key={label} className="portal-assist-row"><span>{label}</span><strong>{value}</strong></div>)}</div>{!portalAssist.portalOpened ? <button className="btn btn-primary" type="button" onClick={()=>{ openPortal(); setPortalAssist((current)=>current ? {...current, portalOpened:true} : current); }}><ExternalLink size={15} strokeWidth={1.8}/> فتح الخدمات الرسمية — أسجل الدخول بنفسي</button> : <><div className="alert-note">بعد تسجيل الدخول والانتقال إلى صفحة عقد الوساطة، اضغط الزر التالي لتجهيز الحقول. إذا تغيّرت البوابة، يتوقف OBR هنا ولا يُدخل بيانات في حقول غير مؤكدة.</div><button className="btn btn-primary" type="button" onClick={preparePortalAssist}><FileText size={15} strokeWidth={1.8}/> تجهيز بيانات العقد</button></>}{portalAssist.result && <div className="portal-assist-result"><h4>الحقول الجاهزة للمراجعة والنسخ</h4>{portalAssist.result.fields?.length ? portalAssist.result.fields.map((field:any)=><div className="portal-assist-row" key={field.key}><span>{field.label}</span><strong>{field.value}</strong><button className="btn btn-outline btn-sm" type="button" onClick={()=>copyPortalValue(field.value)}>نسخ البيانات</button></div>) : <div className="muted">لا توجد حقول مكتملة لتجهيزها؛ أكمل بيانات المسودة أولًا.</div>}<p className="muted">بعد التوثيق الرسمي، عد إلى OBR لاستيراد PDF الموثق أو إدخال رقم العقد الرسمي.</p></div>}</div></div></div>}

    {pdfPreview && <div className="contracts-modal" role="dialog" aria-modal="true" onClick={()=>setPdfPreview(null)}><div className="content-card obr-panel contracts-modal-card" onClick={(e)=>e.stopPropagation()}><div className="card-header"><div className="card-title"><FileText size={17} strokeWidth={1.8}/> {pdfPreview.title}</div><button className="btn btn-outline btn-sm" type="button" onClick={()=>setPdfPreview(null)}><X size={14} strokeWidth={1.8}/> إغلاق</button></div><div className="muted">الملف الأصلي محفوظ دون تعديل: {pdfPreview.name}</div><embed src={pdfPreview.dataUrl} type="application/pdf" style={{width:'100%',height:'65vh',border:'1px solid #e3dcc9',borderRadius:8,background:'#fff'}} /></div></div>}
  </div>;
}

/** عرض مرتب لبيانات العقد: النوع، الأطراف، العقار، المدة، العمولة، التواريخ وبقية الحقول. */
function ContractFields({ c }: { c: any }) {
  const rows: [string, string, boolean][] = [
    ['رقم العقد', c.contract_number || '', false],
    ['النوع', c.contract_type || '', false],
    ['نطاق العقد', c.contract_scope || '', false],
    ['الأطراف', Array.isArray(c.parties) ? c.parties.join(' · ') : (c.parties || ''), false],
    ['العقار', c.property_name || '', false],
    ['العميل', c.client_name || '', false],
    ['المدة', [c.start_date, c.end_date].filter(Boolean).join(' ← '), true],
    ['تاريخ البداية', c.start_date || '', true],
    ['تاريخ النهاية', c.end_date || '', true],
    ['العمولة', [c.commission_amount ?? '', c.commission_mechanism === 'percent' ? '%' : c.currency, COMMISSION_LABEL[c.commission_mechanism] || c.commission_mechanism || ''].filter(Boolean).join(' '), true],
    ['الحالة', STATUS_LABEL[String(c.status || '')] || String(c.status || ''), false],
    ['رقم الترخيص / السجل التجاري', c.license_number || '', false],
    ['المصدر', c.source === 'official_pdf' ? 'PDF موثق' : 'مسودة يدوية', false],
    ['ملف PDF', c.pdf_name || '', false]
  ].filter(([, value]) => String(value).trim().length > 0) as [string, string, boolean][];
  return <div className="table-container"><table className="data-table contract-fields"><tbody>{rows.map(([k,v,num])=><tr key={k}><th scope="row">{k}</th><td className={num ? 'num' : undefined}>{String(v)}</td></tr>)}</tbody></table></div>;
}
function Field({label,children}:{label:string,children:React.ReactNode}){return <div className="form-group"><label>{label}</label>{children}</div>}
function Select({label,value,onChange,items}:{label:string,value:string,onChange:(v:string)=>void,items:any[]}){return <Field label={label}><select className="form-input" value={value||''} onChange={e=>onChange(e.target.value)}><option value="">غير مرتبط</option>{items.map(x=><option key={x.id} value={x.id}>{x.name||x.title||x.number||x.id}</option>)}</select></Field>}
