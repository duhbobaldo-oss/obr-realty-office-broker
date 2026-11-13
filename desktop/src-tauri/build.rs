fn main() {
    // قرار المنتج بعد إعادة تصميم بروتوكول التتبع (Desktop = untrusted public client):
    // لا يُضمَّن أي Secret داخل Desktop EXE إطلاقًا. خدمة التتبع هي مالكة الأسرار،
    // والتطبيق يحدد نفسه بـ installation_id + product_id فقط (تحديد وليس مصادقة)،
    // والخادم يطبّق التحقق والحدود (rate limiting / quotas).
    // OBR_API_KEY لم يعد مستخدمًا من Desktop إطلاقًا ولا يُقرأ من أي مصدر.
    for name in ["OBR_TRACKING_ENDPOINT"] {
        let mut value = std::env::var(name).ok().filter(|v| !v.trim().is_empty());
        if value.is_none() {
            if let Some(home) = std::env::var_os("USERPROFILE") {
                // إعداد غير سري (عنوان الخدمة) يُقرأ من ملف إعداد المستخدم إن وُجد
                let setting = std::path::Path::new(&home)
                    .join(".obr")
                    .join(format!("{name}.txt"));
                if let Ok(content) = std::fs::read_to_string(&setting) {
                    let t = content.trim();
                    if !t.is_empty() {
                        value = Some(t.to_string());
                    }
                }
            }
        }
        if let Some(v) = value {
            println!("cargo:rustc-env={name}={v}");
        }
        println!("cargo:rerun-if-env-changed={name}");
    }
    tauri_build::build()
}
