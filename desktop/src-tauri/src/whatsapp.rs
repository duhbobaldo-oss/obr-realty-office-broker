use std::path::Path;
use std::process::Command;

// منع وميض نافذة Console السوداء عند تشغيل عمليات خارجية من تطبيق GUI.
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[cfg(target_os = "windows")]
fn conceal(mut command: Command) -> Command {
    use std::os::windows::process::CommandExt;
    command.creation_flags(CREATE_NO_WINDOW);
    command
}

#[cfg(not(target_os = "windows"))]
fn conceal(command: Command) -> Command {
    command
}

pub fn normalize_whatsapp_phone(raw: &str) -> Result<String, String> {
    let source = to_ascii_digits(raw).trim().to_string();
    if source.is_empty() {
        return Err("لا يوجد رقم جوال في سجل هذا العميل".into());
    }

    let mut digits: String = source
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '+')
        .collect();
    if digits.starts_with('+') {
        digits = digits[1..].to_string();
    }
    if digits.starts_with("00") {
        digits = digits[2..].to_string();
    }

    if digits.starts_with("05") && digits.len() == 10 {
        digits = format!("966{}", &digits[1..]);
    } else if digits.starts_with('5') && digits.len() == 9 {
        digits = format!("966{digits}");
    } else if digits.starts_with("9660") && digits.len() == 13 {
        digits = format!("966{}", &digits[4..]);
    }

    if digits.len() < 10
        || digits.len() > 15
        || !digits.chars().all(|c| c.is_ascii_digit())
        || digits.starts_with('0')
        || digits.chars().all(|c| c == '0')
    {
        return Err("رقم الجوال غير صالح. استخدم جوالًا سعوديًا صحيحًا أو رقمًا دوليًا كاملًا.".into());
    }

    if digits.starts_with("966") && !(digits.starts_with("9665") && digits.len() == 12) {
        return Err("رقم الجوال السعودي غير صالح".into());
    }

    Ok(digits)
}

pub fn build_whatsapp_url(phone: &str, text: &str) -> Result<String, String> {
    let phone = normalize_whatsapp_phone(phone)?;
    if text.chars().any(|c| c.is_control() && c != '\n' && c != '\r') {
        return Err("نص الرسالة غير صالح".into());
    }
    if text.len() > 2000 {
        return Err("نص الرسالة أطول من المسموح".into());
    }
    let mut url = format!("whatsapp://send?phone={phone}");
    if !text.trim().is_empty() {
        url.push_str("&text=");
        url.push_str(&percent_encode(text));
    }
    Ok(url)
}

pub fn validate_whatsapp_url(url: &str) -> Result<(), String> {
    let rest = url
        .strip_prefix("whatsapp://send?")
        .ok_or_else(|| "رابط غير مسموح".to_string())?;
    let mut phone_ok = false;
    for part in rest.split('&') {
        let (key, value) = part.split_once('=').ok_or_else(|| "رابط غير صالح".to_string())?;
        match key {
            "phone" => {
                if value.len() < 10 || value.len() > 15 || !value.chars().all(|c| c.is_ascii_digit()) {
                    return Err("رقم الجوال غير صالح".into());
                }
                phone_ok = true;
            }
            "text" => {
                if !value.chars().all(|c| {
                    c.is_ascii_alphanumeric() || matches!(c, '%' | '+' | '-' | '_' | '.' | '*' | '~')
                }) {
                    return Err("رابط غير صالح".into());
                }
            }
            _ => return Err("رابط غير مسموح".into()),
        }
    }
    if !phone_ok {
        return Err("رقم الجوال غير صالح".into());
    }
    Ok(())
}

pub fn whatsapp_available() -> bool {
    if protocol_registered("whatsapp") {
        return true;
    }
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        let candidates = [
            format!("{local}\\WhatsApp\\WhatsApp.exe"),
            format!("{local}\\Programs\\WhatsApp\\WhatsApp.exe"),
        ];
        if candidates.iter().any(|path| Path::new(path).exists()) {
            return true;
        }
        if let Ok(entries) = std::fs::read_dir(format!("{local}\\Packages")) {
            if entries.flatten().any(|entry| {
                let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
                name.contains("whatsapp")
            }) {
                return true;
            }
        }
    }
    false
}

pub fn open_whatsapp_chat(phone: String, text: String) -> Result<(), String> {
    let url = build_whatsapp_url(&phone, &text)?;
    crate::validate_open_url(&url)?;
    if !whatsapp_available() {
        return Err("تعذر فتح واتساب. تأكد من تثبيت تطبيق واتساب على هذا الجهاز ثم أعد المحاولة.".into());
    }
    open_windows_url(&url)
}

fn open_windows_url(url: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let safe = url.replace(['\'', '"'], "");
        let ps = conceal(Command::new("powershell"))
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                &format!("Start-Process '{safe}'"),
            ])
            .status();
        if matches!(ps, Ok(status) if status.success()) {
            return Ok(());
        }
        let status = conceal(Command::new("cmd"))
            .args(["/C", &format!("start \"\" \"{safe}\"")])
            .status()
            .map_err(|_| {
                "تعذر فتح واتساب. تأكد من تثبيت تطبيق واتساب على هذا الجهاز ثم أعد المحاولة.".to_string()
            })?;
        if !status.success() {
            return Err("تعذر فتح واتساب. تأكد من تثبيت تطبيق واتساب على هذا الجهاز ثم أعد المحاولة.".into());
        }
        return Ok(());
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = url;
        Err("فتح واتساب غير متاح على هذا النظام".into())
    }
}

fn protocol_registered(scheme: &str) -> bool {
    let keys = [
        format!(r"HKCR\{scheme}"),
        format!(r"HKCU\Software\Classes\{scheme}"),
    ];
    keys.iter().any(|key| {
        conceal(Command::new("reg"))
            .args(["query", key, "/ve"])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    })
}

fn to_ascii_digits(raw: &str) -> String {
    raw.chars()
        .map(|c| match c {
            '٠'..='٩' => char::from_u32(('0' as u32) + (c as u32 - '٠' as u32)).unwrap_or(c),
            '۰'..='۹' => char::from_u32(('0' as u32) + (c as u32 - '۰' as u32)).unwrap_or(c),
            _ => c,
        })
        .collect()
}

fn percent_encode(input: &str) -> String {
    let mut out = String::new();
    for byte in input.as_bytes() {
        match *byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char);
            }
            b' ' => out.push_str("%20"),
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}
