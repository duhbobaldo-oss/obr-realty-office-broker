// ===== فصل سعة البيانات عن سعة التصور =====
// كل تجميعات لوحة العمل تنفذ داخل SQLite (GROUP BY) وتعود إلى React محدودة بـ:
//   - 90 نقطة كحد أقصى لأي سلسلة زمنية
//   - 20 فئة كحد أقصى لأي مخطط تصنيفي
// لا يُرجَع أي سجل خام إلى المخططات؛ الجداول فقط (بترقيم خلفي منفصل).

use rusqlite::{params, params_from_iter, Connection};
use serde_json::{json, Value};

pub const MAX_SERIES_POINTS: usize = 90;
pub const MAX_CATEGORIES: usize = 20;

/// يعيد سطور GROUP BY: {label, value} مع تقييد عدد الفئات.
/// الاستعلام يجب أن يحتوي ?n لكل عنصر في params بالترتيب.
fn group_rows(
    conn: &Connection,
    sql: &str,
    params: &[String],
    value_is_number: bool,
) -> Result<Vec<Value>, String> {
    let mut stmt = conn
        .prepare(sql)
        .map_err(|e| format!("تجميع المخططات: {e}"))?;
    let rows = stmt
        .query_map(params_from_iter(params.iter()), |r| {
            let label: String = r.get(0).unwrap_or_default();
            let value: f64 = if value_is_number {
                r.get::<_, Option<f64>>(1).ok().flatten().unwrap_or(0.0)
            } else {
                r.get::<_, Option<i64>>(1).ok().flatten().unwrap_or(0) as f64
            };
            Ok(json!({ "label": label, "value": value }))
        })
        .map_err(|e| format!("تجميع المخططات: {e}"))?;
    let mut out: Vec<Value> = rows.filter_map(|r| r.ok()).collect();
    out.truncate(MAX_CATEGORIES);
    Ok(out)
}

/// يحدد نطاق التاريخ الوارد في (properties.created_at + market.entry_date)
/// ثم يختار الترجيح تلقائيًا: ≤90 يوماً يومي، ≤630 أسبوعي، وإلا شهري.
fn pick_granularity(conn: &Connection, user_id: &str) -> Result<(String, String, String), String> {
    let min_created: Option<String> = conn
        .query_row(
            "SELECT MIN(created_at) FROM properties WHERE user_id=?1 AND created_at IS NOT NULL",
            params![user_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    let max_created: Option<String> = conn
        .query_row(
            "SELECT MAX(created_at) FROM properties WHERE user_id=?1 AND created_at IS NOT NULL",
            params![user_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    let min_entry: Option<String> = conn
        .query_row(
            "SELECT MIN(entry_date) FROM market_reference_data WHERE user_id=?1 AND entry_date IS NOT NULL",
            params![user_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    let max_entry: Option<String> = conn
        .query_row(
            "SELECT MAX(entry_date) FROM market_reference_data WHERE user_id=?1 AND entry_date IS NOT NULL",
            params![user_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;

    let candidates_min: [Option<String>; 2] = [min_created, min_entry];
    let candidates_max: [Option<String>; 2] = [max_created, max_entry];
    let min_all: Option<String> = candidates_min.iter().flatten().min().cloned();
    let max_all: Option<String> = candidates_max.iter().flatten().max().cloned();
    let min_str = min_all.unwrap_or_default();
    let max_str = max_all.unwrap_or_default();
    let min_date = &min_str[..min_str.len().min(10)];
    let max_date = &max_str[..max_str.len().min(10)];

    // عدد الأيام بين النطاقين عبر julianday
    let days: f64 = conn
        .query_row(
            "SELECT COALESCE(julianday(?2) - julianday(?1), 0)",
            params![min_date, max_date],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;

    let granularity = if days <= 90.0 {
        "day"
    } else if days <= 630.0 {
        "week"
    } else {
        "month"
    };
    Ok((granularity.to_string(), min_str, max_str))
}

/// سلسلة زمنية واحدة داخل SQLite بمقادير يوم/أسبوع/شهر محدودة بـ90 نقطة.
/// column: عمود التاريخ، table: الجدول.
fn time_series(
    conn: &Connection,
    table: &str,
    column: &str,
    granularity: &str,
    user_id: &str,
    value_sql: &str,
) -> Result<Vec<Value>, String> {
    // الأعمدة ثابتة داخل الكود فقط (لا تُمرَّر من المستخدم أبداً)
    let debug_assert_table_ok = matches!(
        table,
        "properties" | "market_reference_data"
    );
    debug_assert!(debug_assert_table_ok);
    if !debug_assert_table_ok {
        return Err("جدول سلسلة زمنية غير مسموح".to_string());
    }
    let bucket = match granularity {
        "week" => format!("date({column}, 'weekday 0', '-6 days')"),
        "month" => format!("substr({column}, 1, 7)"),
        _ => format!("substr({column}, 1, 10)"),
    };
    let sql = format!(
        "SELECT {bucket} AS b, COALESCE(SUM({value_sql}), 0) AS v FROM {table} \
         WHERE user_id=?1 AND {column} IS NOT NULL GROUP BY b ORDER BY b LIMIT {MAX_SERIES_POINTS}"
    );
    let params = [user_id.to_string()];
    let mut stmt = conn.prepare(&sql).map_err(|e| format!("سلسلة زمنية: {e}"))?;
    let rows = stmt
        .query_map(params_from_iter(params.iter()), |r| {
            let label: String = r.get::<_, Option<String>>(0).ok().flatten().unwrap_or_default();
            let value: f64 = r.get::<_, Option<f64>>(1).ok().flatten().unwrap_or(0.0);
            Ok(json!({ "label": label, "value": value }))
        })
        .map_err(|e| format!("سلسلة زمنية: {e}"))?;
    Ok(rows.filter_map(|x| x.ok()).collect())
}

pub fn aggregates(conn: &Connection, user_id: &str, country_override: Option<&str>) -> Result<Value, String> {
    // فلتر الدولة: أولوية للفلتر الوارد من الواجهة، وإلا إعدادات المكتب (نفس نمط contracts::summary)
    let settings_country: String = match country_override {
        Some(c) if !c.trim().is_empty() => c.trim().to_string(),
        _ => conn
            .query_row(
                "SELECT COALESCE(NULLIF(country,''),'') FROM settings WHERE user_id=?1",
                params![user_id],
                |r| r.get(0),
            )
            .unwrap_or_default(),
    };

    let (granularity, range_from, range_to) = pick_granularity(conn, user_id)?;

    // سلاسل اتجاه الإدخال (القائمة + المؤشر) — نقاط محدودة بـ90
    let listings = time_series(conn, "properties", "created_at", &granularity, user_id, "1")?;
    let market_records = time_series(
        conn,
        "market_reference_data",
        "entry_date",
        &granularity,
        user_id,
        "1",
    )?;

    // فلتر الدولة اختياري لبعض التجميعات
    let mut m_params: Vec<String> = vec![user_id.to_string()];
    let m_country_sql = if settings_country.is_empty() {
        String::new()
    } else {
        m_params.push(settings_country.clone());
        " AND country=?2".to_string()
    };

    // تجميعات تصنيفية — الفئات عالية الكاردينالية (المدن) تُختار بأحدث إضافة من SQLite
    // (MAX(entry_date) DESC) وتُحد بأعمال 10 فئات؛ لا يُرجَع كل قاعدة البيانات إلى React.
    let market_cities = group_rows(
        conn,
        "SELECT COALESCE(NULLIF(TRIM(city),''),'—') AS label, COUNT(*) AS v \
         FROM market_reference_data WHERE user_id=?1 GROUP BY label \
         ORDER BY MAX(entry_date) DESC, v DESC LIMIT 10",
        &[user_id.to_string()],
        false,
    )?;

    let market_ppm = group_rows(
        conn,
        &format!(
            "SELECT COALESCE(NULLIF(TRIM(city),''),'—') AS label, AVG(price_per_meter_sale) AS v \
             FROM market_reference_data WHERE user_id=?1 AND price_per_meter_sale > 0{} \
             GROUP BY label ORDER BY MAX(entry_date) DESC, v DESC LIMIT 10",
            m_country_sql.replace("?2", &format!("?{}", m_params.len()))
        ),
        &m_params,
        true,
    )?;

    let campaign_platform = group_rows(
        conn,
        "SELECT COALESCE(NULLIF(TRIM(platform),''),'—') AS label, COALESCE(SUM(inquiries),0) AS v \
         FROM marketing_campaigns WHERE user_id=?1 GROUP BY label ORDER BY v DESC LIMIT 20",
        &[user_id.to_string()],
        false,
    )?;

    // أداء الحملات: Top 20 فقط (النتيجة الكاملة تعرض في جدول صفحة الحملات المُرقم خلفياً)
    let campaign_top = group_rows(
        conn,
        "SELECT COALESCE(NULLIF(TRIM(name),''),'—') AS label, COALESCE(SUM(inquiries),0) AS v \
         FROM marketing_campaigns WHERE user_id=?1 GROUP BY label ORDER BY v DESC LIMIT 20",
        &[user_id.to_string()],
        false,
    )?;
    // التفاصيل الثلاثية (استفسارات/معاينات/صفقات) تُبنى في دالة campaign_top_details المنفصلة

    let client_statuses = group_rows(
        conn,
        "SELECT COALESCE(NULLIF(TRIM(status),''),'—') AS label, COUNT(*) AS v \
         FROM clients WHERE user_id=?1 GROUP BY label ORDER BY v DESC LIMIT 20",
        &[user_id.to_string()],
        false,
    )?;

    let client_reltypes = group_rows(
        conn,
        "SELECT COALESCE(NULLIF(TRIM(c.relationship_type),''),'—') AS label, COUNT(*) AS v \
         FROM clients c WHERE c.user_id=?1 \
           AND NOT EXISTS (SELECT 1 FROM client_roles cr WHERE cr.client_id=c.id) \
         GROUP BY label ORDER BY v DESC LIMIT 20",
        &[user_id.to_string()],
        false,
    )?;

    let client_roles = group_rows(
        conn,
        "SELECT COALESCE(NULLIF(TRIM(role),''),'—') AS label, COUNT(*) AS v \
         FROM client_roles WHERE client_id IN (SELECT id FROM clients WHERE user_id=?1) \
         GROUP BY label ORDER BY v DESC LIMIT 20",
        &[user_id.to_string()],
        false,
    )?;

    let property_statuses = group_rows(
        conn,
        "SELECT COALESCE(NULLIF(TRIM(status),''),'—') AS label, COUNT(*) AS v \
         FROM properties WHERE user_id=?1 GROUP BY label ORDER BY v DESC LIMIT 20",
        &[user_id.to_string()],
        false,
    )?;

    let property_purposes = group_rows(
        conn,
        "SELECT COALESCE(NULLIF(TRIM(purpose),''),'—') AS label, COUNT(*) AS v \
         FROM properties WHERE user_id=?1 GROUP BY label ORDER BY v DESC LIMIT 20",
        &[user_id.to_string()],
        false,
    )?;

    // حالة الفاتورة مُشتقة (لا يوجد عمود status): مدفوع/متأخر/جزئي/مستحق — نفس منطق finance::payment_status
    let invoice_statuses = group_rows(
        conn,
        "SELECT CASE \
           WHEN COALESCE(p.paid_sum,0) >= COALESCE(i.amount,0) THEN 'paid' \
           WHEN COALESCE(i.due_date,'') != '' AND i.due_date < date('now') THEN 'overdue' \
           WHEN COALESCE(p.paid_sum,0) > 0 THEN 'partial' \
           ELSE 'due' END AS label, COUNT(*) AS v \
         FROM finance_invoices i \
         LEFT JOIN (SELECT user_id, target_id, SUM(amount) AS paid_sum FROM finance_payments \
                    WHERE target_type='invoice' GROUP BY user_id, target_id) p \
           ON p.user_id = i.user_id AND p.target_id = i.id \
         WHERE i.user_id=?1 GROUP BY label ORDER BY v DESC LIMIT 20",
        &[user_id.to_string()],
        false,
    )?;

    let mut e_params: Vec<String> = vec![user_id.to_string()];
    let e_country_sql = if settings_country.is_empty() {
        String::new()
    } else {
        e_params.push(settings_country.clone());
        " AND country=?2".to_string()
    };
    let expense_categories = group_rows(
        conn,
        &format!(
            "SELECT COALESCE(NULLIF(TRIM(category),''),'uncategorized') AS label, \
             COALESCE(SUM(amount),0) AS v FROM finance_expenses WHERE user_id=?1{} \
             GROUP BY label ORDER BY v DESC LIMIT 20",
            e_country_sql
        ),
        &e_params,
        true,
    )?;

    let brochure_templates = group_rows(
        conn,
        "SELECT COALESCE(NULLIF(TRIM(template_id),''),'none') AS label, COUNT(*) AS v \
         FROM brochures WHERE user_id=?1 GROUP BY label ORDER BY v DESC LIMIT 20",
        &[user_id.to_string()],
        false,
    )?;

    Ok(json!({
        "trend": {
            "granularity": granularity,
            "range_from": range_from,
            "range_to": range_to,
            "max_points": MAX_SERIES_POINTS,
            "listings": listings,
            "market_records": market_records
        },
        "max_categories": MAX_CATEGORIES,
        "market_cities": market_cities,
        "market_ppm": market_ppm,
        "campaign_platform": campaign_platform,
        "campaign_top": campaign_top,
        "client_statuses": client_statuses,
        "client_reltypes": client_reltypes,
        "client_roles": client_roles,
        "property_statuses": property_statuses,
        "property_purposes": property_purposes,
        "invoice_statuses": invoice_statuses,
        "expense_categories": expense_categories,
        "brochure_templates": brochure_templates
    }))
}

/// تفاصيل الحملات الثلاثية (استفسارات/معاينات/صفقات) — Top 20 فقط.
pub fn campaign_top_details(conn: &Connection, user_id: &str) -> Result<Vec<Value>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT label, inquiries, viewings, deals FROM (\
               SELECT COALESCE(NULLIF(TRIM(name),''),'—') AS label, \
                 COALESCE(SUM(inquiries),0) AS inquiries, \
                 COALESCE(SUM(viewings),0) AS viewings, \
                 COALESCE(SUM(deals),0) AS deals \
               FROM marketing_campaigns WHERE user_id=?1 GROUP BY label) \
             ORDER BY inquiries DESC LIMIT 20",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![user_id], |r| {
            let label: String = r.get::<_, Option<String>>(0).ok().flatten().unwrap_or_default();
            let inquiries: f64 = r.get::<_, Option<f64>>(1).ok().flatten().unwrap_or(0.0);
            let viewings: f64 = r.get::<_, Option<f64>>(2).ok().flatten().unwrap_or(0.0);
            let deals: f64 = r.get::<_, Option<f64>>(3).ok().flatten().unwrap_or(0.0);
            Ok(json!({ "label": label, "inquiries": inquiries, "viewings": viewings, "deals": deals }))
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(|x| x.ok()).collect())
}
