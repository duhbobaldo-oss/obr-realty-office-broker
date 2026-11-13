// عميل API - يستدعي Tauri Commands محليًا (Offline بالكامل)
// يحافظ على نفس الواجهة السابقة (path, options) حتى لا تتغير أي صفحة

import { invoke } from '@tauri-apps/api/core';

// خريطة مسارات API إلى أسماء Tauri Commands
// كل مسار يتوافق مع command في Rust backend
const ROUTE_MAP: Record<string, { command: string; method?: string }> = {
  // المصادقة المحلية (بدون حساب OBR)
  '/api/auth/me': { command: 'auth_me' },
  '/api/auth/ensure-local': { command: 'auth_ensure_local', method: 'POST' },

  // العروض
  '/api/properties': { command: 'properties_list' },
  'POST /api/properties': { command: 'properties_create', method: 'POST' },
  '/api/properties/:id': { command: 'properties_get' },
  'PUT /api/properties/:id': { command: 'properties_update', method: 'PUT' },
  'DELETE /api/properties/:id': { command: 'properties_delete', method: 'DELETE' },
  '/api/properties/:id/duplicate': { command: 'properties_duplicate', method: 'POST' },
  '/api/properties/:id/status': { command: 'properties_change_status', method: 'PATCH' },
  '/api/properties/:id/updates': { command: 'properties_add_update', method: 'POST' },
  '/api/properties/:id/images': { command: 'properties_images' },
  'POST /api/properties/:id/images': { command: 'properties_add_image', method: 'POST' },
  'POST /api/properties/images/read': { command: 'property_image_read', method: 'POST' },
  'DELETE /api/properties/:id/images/:imageId': { command: 'properties_delete_image', method: 'DELETE' },

  // العملاء
  '/api/clients': { command: 'clients_list' },
  '/api/clients/options': { command: 'clients_options' },
  'POST /api/clients': { command: 'clients_create', method: 'POST' },
  '/api/clients/:id': { command: 'clients_get' },
  'PUT /api/clients/:id': { command: 'clients_update', method: 'PUT' },
  'DELETE /api/clients/:id': { command: 'clients_delete', method: 'DELETE' },
  '/api/clients/:id/communications': { command: 'clients_add_communication', method: 'POST' },
  '/api/clients/:id/link-property': { command: 'clients_link_property', method: 'POST' },
  '/api/clients/:id/unlink-property/:propertyId': { command: 'clients_unlink_property', method: 'DELETE' },

  // المؤشر
  '/api/market': { command: 'market_list' },
  'POST /api/market': { command: 'market_create', method: 'POST' },
  '/api/market/stats': { command: 'market_stats' },
  '/api/market/trends': { command: 'market_trends' },
  '/api/market/filter-cities': { command: 'market_filter_cities' },
  '/api/market/filter-districts': { command: 'market_filter_districts' },
  '/api/market/geo-summary': { command: 'market_geo_summary' },
  '/api/market/disclaimer': { command: 'market_disclaimer' },
  '/api/market/bulk': { command: 'market_bulk_create', method: 'POST' },
  'POST /api/market/purge-audit': { command: 'market_purge_audit', method: 'POST' },
  'POST /api/market/delete-many': { command: 'market_delete_many', method: 'POST' },
  'DELETE /api/market/records': { command: 'market_delete_filtered', method: 'DELETE' },
  'PUT /api/market/:id': { command: 'market_update', method: 'PUT' },
  'DELETE /api/market/:id': { command: 'market_delete', method: 'DELETE' },

  // الحملات التسويقية
  '/api/campaigns': { command: 'campaigns_list' },
  'POST /api/campaigns': { command: 'campaigns_create', method: 'POST' },
  '/api/campaigns/stats': { command: 'campaigns_stats' },
  '/api/campaigns/tracking/status': { command: 'campaigns_tracking_status' },
  '/api/campaigns/:id': { command: 'campaigns_get' },
  'PUT /api/campaigns/:id': { command: 'campaigns_update', method: 'PUT' },
  'DELETE /api/campaigns/:id': { command: 'campaigns_delete', method: 'DELETE' },
  'POST /api/campaigns/:id/tracking/enable': { command: 'campaigns_tracking_enable', method: 'POST' },
  'POST /api/campaigns/:id/tracking/disable': { command: 'campaigns_tracking_disable', method: 'POST' },
  'POST /api/campaigns/:id/tracking/sync': { command: 'campaigns_tracking_sync', method: 'POST' },

  // المواعيد
  '/api/appointments': { command: 'appointments_list' },
  'POST /api/appointments': { command: 'appointments_create', method: 'POST' },
  '/api/appointments/:id': { command: 'appointments_get' },
  'PUT /api/appointments/:id': { command: 'appointments_update', method: 'PUT' },
  'DELETE /api/appointments/:id': { command: 'appointments_delete', method: 'DELETE' },

  // المالية والعقود
  '/api/finance/summary': { command: 'finance_summary' },
  '/api/finance/ledger': { command: 'finance_ledger' },
  '/api/finance/receivables': { command: 'finance_receivables' },
  '/api/finance/operations': { command: 'finance_operations_list' },
  'POST /api/finance/operations': { command: 'finance_operations_create', method: 'POST' },
  '/api/finance/operations/:id': { command: 'finance_operations_get' },
  'PUT /api/finance/operations/:id': { command: 'finance_operations_update', method: 'PUT' },
  'DELETE /api/finance/operations/:id': { command: 'finance_operations_delete', method: 'DELETE' },
  '/api/finance/invoices': { command: 'finance_invoices_list' },
  'POST /api/finance/invoices': { command: 'finance_invoices_create', method: 'POST' },
  '/api/finance/invoices/:id/pdf': { command: 'finance_invoice_pdf', method: 'POST' },
  'POST /api/finance/record/pdf': { command: 'finance_record_pdf', method: 'POST' },
  '/api/finance/invoices/:id': { command: 'finance_invoices_get' },
  'PUT /api/finance/invoices/:id': { command: 'finance_invoices_update', method: 'PUT' },
  'DELETE /api/finance/invoices/:id': { command: 'finance_invoices_delete', method: 'DELETE' },
  '/api/finance/commissions': { command: 'finance_commissions_list' },
  'POST /api/finance/commissions': { command: 'finance_commissions_create', method: 'POST' },
  '/api/finance/commissions/:id': { command: 'finance_commissions_get' },
  'PUT /api/finance/commissions/:id': { command: 'finance_commissions_update', method: 'PUT' },
  'DELETE /api/finance/commissions/:id': { command: 'finance_commissions_delete', method: 'DELETE' },
  '/api/finance/expenses': { command: 'finance_expenses_list' },
  'POST /api/finance/expenses': { command: 'finance_expenses_create', method: 'POST' },
  '/api/finance/expenses/:id': { command: 'finance_expenses_get' },
  'PUT /api/finance/expenses/:id': { command: 'finance_expenses_update', method: 'PUT' },
  'DELETE /api/finance/expenses/:id': { command: 'finance_expenses_delete', method: 'DELETE' },
  '/api/finance/payments': { command: 'finance_payments_list' },
  'POST /api/finance/payments': { command: 'finance_payments_create', method: 'POST' },
  '/api/finance/payments/:id': { command: 'finance_payments_get' },
  'PUT /api/finance/payments/:id': { command: 'finance_payments_update', method: 'PUT' },
  'DELETE /api/finance/payments/:id': { command: 'finance_payments_delete', method: 'DELETE' },
  'POST /api/finance/report/pdf': { command: 'finance_report_pdf', method: 'POST' },
  'POST /api/finance/open-pdf': { command: 'finance_open_pdf', method: 'POST' },
  '/api/contracts': { command: 'contracts_list' },
  'POST /api/contracts': { command: 'contracts_save', method: 'POST' },
  'DELETE /api/contracts/:id': { command: 'contracts_delete', method: 'DELETE' },
  '/api/contracts/summary': { command: 'contracts_summary' },
  'POST /api/contracts/preview-pdf': { command: 'contracts_preview_pdf', method: 'POST' },
  'POST /api/contracts/portal-assist': { command: 'contracts_portal_assist', method: 'POST' },
  '/api/contracts/:id/pdf-data': { command: 'contracts_pdf_data' },
  'POST /api/contracts/:id/open-pdf': { command: 'contracts_open_pdf', method: 'POST' },
  'POST /api/contracts/print-pdf': { command: 'contracts_print_pdf', method: 'POST' },

  // الإعدادات
  '/api/settings': { command: 'settings_get' },
  'POST /api/official-integrations': { command: 'official_integrations_request', method: 'POST' },
  'PUT /api/settings': { command: 'settings_update', method: 'PUT' },

  // لوحة العمل
  '/api/dashboard/stats': { command: 'dashboard_stats' },
  '/api/dashboard/activity': { command: 'dashboard_activity' },
  '/api/dashboard/aggregates': { command: 'dashboard_aggregates' },

  // البروشرات / تصميم العروض
  '/api/brochures': { command: 'brochures_list' },
  'POST /api/brochures': { command: 'brochures_create', method: 'POST' },
  '/api/brochures/templates': { command: 'brochures_templates' },
  '/api/brochures/:id': { command: 'brochures_get' },
  'PUT /api/brochures/:id': { command: 'brochures_update', method: 'PUT' },
  'DELETE /api/brochures/:id': { command: 'brochures_delete', method: 'DELETE' },

  // التصدير
  '/api/export/all': { command: 'export_all' },
  '/api/export/properties': { command: 'export_properties' },
  '/api/export/clients': { command: 'export_clients' },
  '/api/export/market': { command: 'export_market' },

  // النظام والنسخ الاحتياطي
  '/api/system/info': { command: 'system_info' },
  'POST /api/backup/create': { command: 'backup_create', method: 'POST' },
  '/api/backup/list': { command: 'backup_list' },
  'POST /api/backup/restore': { command: 'backup_restore', method: 'POST' },
  'POST /api/backup/delete': { command: 'backup_delete', method: 'POST' },
  'POST /api/import/json': { command: 'import_data_json', method: 'POST' },

  // بيانات تدريبية (صريح — لا تُزرع تلقائياً)
  '/api/demo/status': { command: 'demo_status' },
  'POST /api/demo/delete': { command: 'demo_delete', method: 'POST' },
  'POST /api/demo/reseed': { command: 'demo_reseed', method: 'POST' },
  'POST /api/demo/prepare': { command: 'demo_prepare', method: 'POST' },
};

// استخراج المعرّفات من المسار (مثل :id)
function extractParams(path: string, pattern: string): Record<string, string> {
  const pathParts = path.split('/');
  const patternParts = pattern.split('/');
  const params: Record<string, string> = {};

  for (let i = 0; i < patternParts.length; i++) {
    if (patternParts[i].startsWith(':')) {
      const key = patternParts[i].slice(1);
      params[key] = pathParts[i] || '';
    }
  }
  return params;
}

// مطابقة المسار مع النمط (مدركة لطريقة الطلب GET/POST/PUT...)
function matchRoute(path: string, method = 'GET'): { command: string; method?: string; params: Record<string, string> } | null {
  // إزالة query string
  const cleanPath = path.split('?')[0];
  const M = method.toUpperCase();

  // محاولة المطابقة الدقيقة أولاً (مفتاح مسبق بالطريقة له الأولوية)
  if (ROUTE_MAP[`${M} ${cleanPath}`]) {
    return { ...ROUTE_MAP[`${M} ${cleanPath}`], params: {} };
  }
  if (ROUTE_MAP[cleanPath]) {
    const r = ROUTE_MAP[cleanPath];
    // المسار غير المسبوق يقبل GET أو أي طريقة إذا لم تحدد
    if (!r.method || r.method === M) {
      return { ...r, params: {} };
    }
  }

  // محاولة المطابقة مع المعرّفات
  for (const [pattern, route] of Object.entries(ROUTE_MAP)) {
    const prefixed = pattern.startsWith(`${M} `);
    const barePattern = prefixed ? pattern.slice(M.length + 1) : pattern;
    if (!barePattern.includes(':')) continue;
    // إن كان المفتاح مسبوقًا بطريقة فيجب أن تطابق؛ وإن لم يكن مسبوقًا وطريقته مختلفة فتخطَّ
    // المسار غير المسبوق بدون method = GET فقط.
    // غير ذلك كان PUT /api/clients/:id يُطابق clients_get فيبدو الحفظ ناجحًا دون كتابة.
    if (prefixed ? route.method !== M : (route.method ? route.method !== M : M !== 'GET')) continue;

    const patternParts = barePattern.split('/');
    const pathParts = cleanPath.split('/');
    if (patternParts.length !== pathParts.length) continue;

    let match = true;
    for (let i = 0; i < patternParts.length; i++) {
      if (patternParts[i].startsWith(':')) continue;
      if (patternParts[i] !== pathParts[i]) {
        match = false;
        break;
      }
    }
    if (match) {
      return { ...route, params: extractParams(cleanPath, barePattern) };
    }
  }

  return null;
}

// تحويل query string إلى كائن
function parseQuery(path: string): Record<string, string> {
  const queryStr = path.split('?')[1];
  if (!queryStr) return {};
  const params: Record<string, string> = {};
  for (const pair of queryStr.split('&')) {
    const [key, value] = pair.split('=');
    if (key) params[decodeURIComponent(key)] = decodeURIComponent(value || '');
  }
  return params;
}

/** Tauri 2 يحوّل وسائط الأوامر إلى camelCase — نرسل النسختين حتى لا تُفقد الحقول مثل image_url */
function snakeToCamel(key: string): string {
  return key.replace(/_([a-z])/g, (_, ch) => ch.toUpperCase());
}

function assignInvokeArg(args: Record<string, unknown>, key: string, value: unknown) {
  args[key] = value;
  const camel = snakeToCamel(key);
  if (camel !== key && args[camel] === undefined) {
    args[camel] = value;
  }
}

/** أوامر Rust تستخدم comm_type بينما الواجهة ترسل type */
export function applyCommandArgAliases(command: string, args: Record<string, unknown>) {
  if (command === 'clients_add_communication') {
    const commType = args.commType ?? args.comm_type ?? args.type;
    if (typeof commType === 'string') {
      args.commType = commType;
      args.comm_type = commType;
    }
    if (args.subject === undefined) args.subject = '';
    if (args.notes === undefined) args.notes = '';
  }
  return args;
}

export async function api<T = any>(
  path: string,
  options: { method?: string; body?: any; auth?: boolean } = {}
): Promise<T> {
  const { method = 'GET', body, auth = true } = options;

  // الحصول على التوكين من localStorage
  let token = '';
  if (auth) {
    const saved = localStorage.getItem('mawasib_auth');
    if (saved) {
      try {
        const { token: t } = JSON.parse(saved);
        token = t || '';
      } catch {
        // تجاهل
      }
    }
  }

  // مطابقة المسار مع command
  const route = matchRoute(path, method);
  if (!route) {
    throw new Error(`المسار غير مدعوم: ${path}`);
  }

  const query = parseQuery(path);
  const args: Record<string, any> = {};

  // إضافة التوكين
  if (auth && token) {
    args.token = token;
  }

  // دمج بيانات body في المستوى الأعلى
  // (بعض أوامر Tauri تتوقع حقولًا مسطحة مثل username/password، وبعضها يتوقع data)
  if (body !== undefined && body !== null && typeof body === 'object' && !Array.isArray(body)) {
    for (const [key, value] of Object.entries(body)) {
      if (value !== undefined) {
        assignInvokeArg(args, key, value);
      }
    }
    args.data = body;
  } else if (body !== undefined) {
    args.data = body;
  }

  // إضافة معرّفات المسار (لها الأولوية على body)
  for (const [key, value] of Object.entries(route.params)) {
    assignInvokeArg(args, key, value);
  }

  // إضافة معاملات query (لها الأولوية أيضًا)
  // الأرقام تُحوَّل من نص إلى رقم لأن أوامر Rust تتوقع أنواعًا صحيحة
  for (const [key, value] of Object.entries(query)) {
    if (value === '') continue;
    // البحث يصل دائمًا كنص حتى لو كان أرقامًا (76، 0551234567) لتفادي Type Error في Tauri
    if (key === 'search' || key.endsWith('_search')) {
      args[key] = String(value);
      continue;
    }
    const n = Number(value);
    args[key] = value !== '' && !isNaN(n) && /^-?\d+(\.\d+)?$/.test(value) ? n : value;
  }

  applyCommandArgAliases(route.command, args);

  // استدعاء Tauri command
  try {
    const result = await invoke(route.command, args);
    return result as T;
  } catch (err: any) {
    // Tauri يرمي الأخطاء كـ string
    const message = typeof err === 'string' ? err : (err?.message || 'حدث خطأ في الاتصال');
    throw new Error(message);
  }
}
