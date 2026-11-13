//! Configured HTTP connectivity only. No authority-specific business actions.
use base64::{engine::general_purpose::STANDARD, Engine};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::BTreeMap, io::Read, time::Duration};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Integration {
    #[serde(default)] pub id: String,
    pub country: String,
    #[serde(default)] pub jurisdiction: String,
    pub authority_name: String,
    pub integration_name: String,
    pub mode: String,
    #[serde(default)] pub service_url: String,
    #[serde(default)] pub base_url: String,
    #[serde(default)] pub endpoint: String,
    #[serde(default = "get_method")] pub http_method: String,
    #[serde(default = "no_auth")] pub auth_type: String,
    #[serde(default)] pub api_key_header: String,
    #[serde(default)] pub client_id: String,
    #[serde(default)] pub token_url: String,
    #[serde(default)] pub scope: String,
    #[serde(default)] pub enabled: bool,
    #[serde(default)] pub status: String,
    #[serde(default)] pub last_tested_at: Option<String>,
    #[serde(default)] pub has_secrets: bool,
}
fn get_method() -> String { "GET".into() }
fn no_auth() -> String { "none".into() }

// Deliberately neither Debug nor Serialize: never returned to the UI or logs.
#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Secrets {
    #[serde(default)] api_key: String,
    #[serde(default)] token: String,
    #[serde(default)] username: String,
    #[serde(default)] password: String,
    #[serde(default)] client_secret: String,
    #[serde(default)] custom_headers: BTreeMap<String, String>,
}

pub fn list(conn: &Connection, user: &str) -> Result<Value, String> {
    let mut stmt = conn.prepare("SELECT config FROM official_integrations WHERE user_id=?1 ORDER BY id").map_err(|_| "integration.storage")?;
    let rows = stmt.query_map([user], |r| r.get::<_, String>(0)).map_err(|_| "integration.storage")?;
    let mut items = Vec::new();
    for row in rows { items.push(serde_json::from_str::<Integration>(&row.map_err(|_| "integration.storage")?).map_err(|_| "integration.storage")?); }
    Ok(json!({"items":items}))
}

fn read(conn: &Connection, user: &str, id: &str) -> Result<(Integration, Vec<u8>), String> {
    let row: Option<(String, Vec<u8>)> = conn.query_row("SELECT config, secrets FROM official_integrations WHERE user_id=?1 AND id=?2", params![user,id], |r| Ok((r.get(0)?,r.get(1)?))).optional().map_err(|_| "integration.storage")?;
    let (config, encrypted) = row.ok_or("integration.not_found")?;
    Ok((serde_json::from_str(&config).map_err(|_| "integration.storage")?, encrypted))
}

fn checked_url(raw: &str, api: bool) -> Result<tauri::Url, String> {
    let url = tauri::Url::parse(raw).map_err(|_| "integration.https_required")?;
    let secure = url.scheme() == "https";
    #[cfg(test)] let secure = secure || (url.scheme() == "http" && url.host_str() == Some("127.0.0.1"));
    if !secure || url.host_str().is_none() || !url.username().is_empty() || url.password().is_some() || url.fragment().is_some() || (api && url.query().is_some()) {
        return Err("integration.https_required".into());
    }
    Ok(url)
}

fn header_ok(name: &str, value: &str) -> bool {
    !name.is_empty() && name.bytes().all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b))
        && !value.contains(['\r','\n']) && !matches!(name.to_ascii_lowercase().as_str(), "host"|"content-length"|"transfer-encoding"|"connection"|"proxy-authorization")
}

fn api_url(item: &Integration) -> Result<tauri::Url, String> {
    let base = checked_url(&item.base_url, true)?;
    let target = base.join(&item.endpoint).map_err(|_| "integration.endpoint")?;
    checked_url(target.as_str(), true)?;
    if target.origin() != base.origin() { return Err("integration.endpoint".into()); }
    Ok(target)
}

fn validate(item: &Integration, secrets: &Secrets) -> Result<(), String> {
    let profile = crate::country::profile(&item.country)?;
    if !item.jurisdiction.is_empty() && (item.country != "AE" || !profile["licenses"].as_array().unwrap().iter().any(|l| l["jurisdiction"] == item.jurisdiction)) { return Err("integration.jurisdiction_error".into()); }
    if item.authority_name.trim().is_empty() || item.integration_name.trim().is_empty() { return Err("integration.names_required".into()); }
    if item.mode == "official_url" { checked_url(&item.service_url, false)?; return Ok(()); }
    if item.mode != "api" { return Err("integration.mode".into()); }
    api_url(item)?;
    if !matches!(item.http_method.as_str(), "GET"|"HEAD"|"POST"|"PUT"|"PATCH"|"DELETE"|"OPTIONS") { return Err("integration.method".into()); }
    let auth_ok = match item.auth_type.as_str() {
        "none" => true,
        "api_key" => header_ok(&item.api_key_header, &secrets.api_key) && !secrets.api_key.is_empty(),
        "bearer" => !secrets.token.is_empty() && header_ok("Authorization", &secrets.token),
        "basic" => !secrets.username.is_empty() && !secrets.username.contains(':') && !secrets.password.is_empty(),
        "oauth2" => { checked_url(&item.token_url, true)?; !item.client_id.is_empty() && !secrets.client_secret.is_empty() },
        _ => false,
    };
    if !auth_ok || secrets.custom_headers.iter().any(|(k,v)| !header_ok(k,v) || k.eq_ignore_ascii_case("authorization") || (item.auth_type == "api_key" && k.eq_ignore_ascii_case(&item.api_key_header))) { return Err("integration.credentials".into()); }
    Ok(())
}

pub fn save(conn: &Connection, user: &str, data: &Value) -> Result<Value, String> {
    let mut item: Integration = serde_json::from_value(data["integration"].clone()).map_err(|_| "integration.invalid")?;
    let old = if item.id.is_empty() { None } else { Some(read(conn,user,&item.id)?) };
    let encrypted = if let Some(secret_value) = data.get("secrets").filter(|s| !s.is_null()) {
        let _: Secrets = serde_json::from_value(secret_value.clone()).map_err(|_| "integration.credentials")?;
        let mut bytes = serde_json::to_vec(secret_value).map_err(|_| "integration.credentials")?;
        let result = crate::integration_secrets::protect(&bytes); bytes.fill(0); result?
    } else { old.as_ref().map(|(_,s)| s.clone()).unwrap_or_default() };
    let secrets = decode(&encrypted)?;
    validate(&item,&secrets)?;
    if item.id.is_empty() { item.id = uuid::Uuid::new_v4().to_string(); }
    // One active choice per country/jurisdiction; explicit conflict rather than arbitrary priority.
    if item.enabled {
        let items = list(conn,user)?;
        if items["items"].as_array().unwrap().iter().any(|i| i["id"] != item.id && i["country"] == item.country && i["jurisdiction"] == item.jurisdiction && i["enabled"] == true) { return Err("integration.active_conflict".into()); }
    }
    item.has_secrets = !encrypted.is_empty();
    item.status = "untested".into(); item.last_tested_at = None;
    if let Some((previous, _)) = &old {
        let mut comparable = item.clone();
        comparable.enabled = previous.enabled;
        comparable.status = previous.status.clone();
        comparable.last_tested_at = previous.last_tested_at.clone();
        if data.get("secrets").filter(|s| !s.is_null()).is_none() && serde_json::to_value(&comparable).ok() == serde_json::to_value(previous).ok() {
            item.status = previous.status.clone(); item.last_tested_at = previous.last_tested_at.clone();
        }
    }
    let config = serde_json::to_string(&item).map_err(|_| "integration.storage")?;
    conn.execute("INSERT INTO official_integrations(id,user_id,config,secrets) VALUES(?1,?2,?3,?4) ON CONFLICT(id) DO UPDATE SET config=excluded.config,secrets=excluded.secrets WHERE official_integrations.user_id=excluded.user_id", params![item.id,user,config,encrypted]).map_err(|_| "integration.storage")?;
    Ok(json!(item))
}

fn decode(encrypted: &[u8]) -> Result<Secrets, String> {
    if encrypted.is_empty() { return Ok(Secrets::default()); }
    let mut bytes = crate::integration_secrets::unprotect(encrypted)?;
    let result = serde_json::from_slice(&bytes).map_err(|_| "integration.credentials".into()); bytes.fill(0); result
}

pub fn delete(conn: &Connection, user: &str, id: &str) -> Result<Value, String> {
    conn.execute("DELETE FROM official_integrations WHERE user_id=?1 AND id=?2", params![user,id]).map_err(|_| "integration.storage")?;
    Ok(json!({"ok":true}))
}

/// A bounded client with TLS validation and no redirects (credentials never follow a redirect).
pub struct Connector { item: Integration, secrets: Secrets, agent: reqwest::blocking::Client }
impl Connector {
    pub fn load(conn: &Connection, user: &str, id: &str) -> Result<Self, String> {
        let (item, encrypted) = read(conn,user,id)?;
        let secrets = decode(&encrypted)?; validate(&item,&secrets)?;
        Ok(Self { item, secrets, agent: reqwest::blocking::Client::builder().redirect(reqwest::redirect::Policy::none()).timeout(Duration::from_secs(15)).connect_timeout(Duration::from_secs(10)).build().map_err(|_| "integration.failed")? })
    }
    /// Reusable request builder. Mapping-specific code must explicitly supply any business body.
    pub fn request(&self) -> Result<reqwest::blocking::RequestBuilder, String> {
        let url = if self.item.mode == "api" { api_url(&self.item)? } else { checked_url(&self.item.service_url,false)? };
        let method = if self.item.mode == "api" { self.item.http_method.as_str() } else { "GET" };
        let method = reqwest::Method::from_bytes(method.as_bytes()).map_err(|_| "integration.method")?;
        let mut request = self.agent.request(method,url.as_str());
        if self.item.mode == "official_url" { return Ok(request); }
        for (name,value) in &self.secrets.custom_headers { request = secret_header(request, name, value)?; }
        request = match self.item.auth_type.as_str() {
            "api_key" => secret_header(request, &self.item.api_key_header, &self.secrets.api_key)?,
            "bearer" => secret_header(request, "Authorization", &format!("Bearer {}", self.secrets.token))?,
            "basic" => secret_header(request, "Authorization", &format!("Basic {}",STANDARD.encode(format!("{}:{}",self.secrets.username,self.secrets.password))))?,
            "oauth2" => {
                let response = self.agent.post(&self.item.token_url).form(&[("grant_type","client_credentials"),("client_id",&self.item.client_id),("client_secret",&self.secrets.client_secret),("scope",&self.item.scope)]).send().and_then(reqwest::blocking::Response::error_for_status).map_err(|_| "integration.auth_failed")?;
                let mut body = String::new(); response.take(65537).read_to_string(&mut body).map_err(|_| "integration.auth_failed")?;
                if body.len() > 65536 { return Err("integration.auth_failed".into()); }
                let value: Value = serde_json::from_str(&body).map_err(|_| "integration.auth_failed")?;
                let token = value["access_token"].as_str().filter(|v| !v.is_empty() && header_ok("Authorization",v)).ok_or("integration.auth_failed")?;
                if value["token_type"].as_str().map(|s| !s.eq_ignore_ascii_case("bearer")).unwrap_or(false) { return Err("integration.auth_failed".into()); }
                secret_header(request, "Authorization", &format!("Bearer {token}"))?
            },
            _ => request,
        };
        Ok(request)
    }
}

fn secret_header(request: reqwest::blocking::RequestBuilder, name: &str, value: &str) -> Result<reqwest::blocking::RequestBuilder, String> {
    let name = reqwest::header::HeaderName::from_bytes(name.as_bytes()).map_err(|_| "integration.credentials")?;
    let mut value = reqwest::header::HeaderValue::from_str(value).map_err(|_| "integration.credentials")?;
    value.set_sensitive(true);
    Ok(request.header(name, value))
}

pub fn test_connection(conn: &Connection, user: &str, id: &str) -> Result<Value, String> {
    let connector = Connector::load(conn,user,id)?;
    // Test is an explicitly requested probe to the configured endpoint; never a contract action.
    let original_config = serde_json::to_string(&connector.item).map_err(|_| "integration.storage")?;
    let outcome = connector.request().and_then(|r| r.send().map_err(|_| "integration.connection_failed".into()));
    let success = outcome.as_ref().map(|r| r.status().is_success()).unwrap_or(false);
    let mut item = connector.item; item.status = if success { "success" } else { "failure" }.into(); item.last_tested_at = Some(chrono::Utc::now().to_rfc3339());
    let updated = conn.execute("UPDATE official_integrations SET config=?1 WHERE user_id=?2 AND id=?3 AND config=?4",params![serde_json::to_string(&item).map_err(|_| "integration.storage")?,user,id,original_config]).map_err(|_| "integration.storage")?;
    if updated != 1 { return Err("integration.changed".into()); }
    Ok(json!({"integration":item,"message":if success { if item.mode == "api" { "integration.connectedMapping" } else { "integration.connected" } } else { "integration.connection_failed" }}))
}

pub fn resolve(conn: &Connection, user: &str, country: &str, license_type: &str) -> Result<Value,String> {
    let profile = crate::country::profile(country)?;
    let jurisdiction = profile["licenses"].as_array().unwrap().iter().find(|l| l["id"] == license_type).and_then(|l| l["jurisdiction"].as_str()).unwrap_or("");
    let items = list(conn,user)?;
    let mut matches: Vec<&Value> = items["items"].as_array().unwrap().iter().filter(|i| i["country"] == country && i["enabled"] == true && (i["jurisdiction"] == jurisdiction || i["jurisdiction"] == "")).collect();
    matches.sort_by_key(|i| if !jurisdiction.is_empty() && i["jurisdiction"] == jurisdiction {0} else {1});
    if let Some(i) = matches.first() { return Ok(json!({"mode":i["mode"],"url":i["serviceUrl"],"integrationId":i["id"],"status":i["status"],"message":"integration.mapping_required"})); }
    let fallback = crate::country::service(country,license_type)?;
    Ok(json!({"mode":"official_url","url":fallback["url"],"fallback":true}))
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    fn database() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE users(id TEXT PRIMARY KEY); INSERT INTO users VALUES('u');").unwrap();
        conn.execute_batch(crate::db::migrations::MIGRATIONS.iter().find(|m| m.version == 16).unwrap().sql).unwrap();
        conn
    }
    fn config(url: &str, mode: &str) -> Value {
        json!({"country":"SA","authorityName":"Test authority","integrationName":"Test integration","mode":mode,"serviceUrl":url,"baseUrl":url,"endpoint":"/probe","enabled":false})
    }
    fn server(responses: Vec<&'static str>, inspect: impl Fn(usize,&str) + Send + 'static) -> (String, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = format!("http://{}",listener.local_addr().unwrap());
        listener.set_nonblocking(true).unwrap();
        let thread = std::thread::spawn(move || {
            for (index,response) in responses.into_iter().enumerate() {
                let until = std::time::Instant::now() + Duration::from_secs(20);
                let mut stream = loop {
                    if let Ok((s,_)) = listener.accept() { break s; }
                    assert!(std::time::Instant::now() < until,"probe request not received");
                    std::thread::sleep(Duration::from_millis(10));
                };
                // على Windows يرث المقبس المقبول اللاحجب من الـlistener (FIONBIO)، فيعود
                // read فورًا بـWSAEWOULDBLOCK قبل وصول الطلب — نُعيده blocking ثم نضبط المهلة.
                stream.set_nonblocking(false).unwrap();
                stream.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
                let mut bytes = Vec::new();
                loop {
                    let mut chunk=[0;4096]; let n=stream.read(&mut chunk).unwrap(); if n==0 {break;}
                    bytes.extend_from_slice(&chunk[..n]);
                    if let Some(end)=bytes.windows(4).position(|w| w==b"\r\n\r\n") {
                        let header=String::from_utf8_lossy(&bytes[..end]);
                        let len=header.lines().find_map(|l| l.to_lowercase().strip_prefix("content-length:").and_then(|v|v.trim().parse::<usize>().ok())).unwrap_or(0);
                        if bytes.len()>=end+4+len {break;}
                    }
                }
                inspect(index,&String::from_utf8_lossy(&bytes));
                stream.write_all(response.as_bytes()).unwrap();
            }
        });
        (address,thread)
    }
    const OK: &str = "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nOK";

    #[test]
    fn official_url_save_probe_resolve_disable_and_country_retention() {
        let conn=database();
        let (url,worker)=server(vec![OK],|_,req| assert!(req.starts_with("GET /")));
        let mut value=config(&url,"official_url"); value["enabled"]=json!(true);
        let item=save(&conn,"u",&json!({"integration":value})).unwrap();
        let id=item["id"].as_str().unwrap();
        assert_eq!(resolve(&conn,"u","SA","brokerage_marketing").unwrap()["url"],url);
        assert_eq!(test_connection(&conn,"u",id).unwrap()["integration"]["status"],"success"); worker.join().unwrap();
        let mut current=read(&conn,"u",id).unwrap().0; current.enabled=false;
        let disabled=save(&conn,"u",&json!({"integration":current})).unwrap();
        assert_eq!(disabled["status"],"success");
        assert_eq!(resolve(&conn,"u","SA","brokerage_marketing").unwrap()["fallback"],true);
        let mut ae=config("https://example.com/ae","official_url");ae["country"]=json!("AE");ae["jurisdiction"]=json!("دبي");ae["enabled"]=json!(true);
        save(&conn,"u",&json!({"integration":ae})).unwrap();
        assert_eq!(list(&conn,"u").unwrap()["items"].as_array().unwrap().len(),2);
        assert_eq!(resolve(&conn,"u","AE","ae_dubai_broker").unwrap()["url"],"https://example.com/ae");
        assert_eq!(resolve(&conn,"u","AE","ae_abudhabi_broker").unwrap()["fallback"],true);
        assert!(read(&conn,"another",id).is_err());
    }

    #[test]
    fn official_api_dpapi_and_real_auth_probes() {
        let conn=database();
        for auth in ["none","api_key","bearer","basic","oauth2"] {
            let responses=if auth=="oauth2" { vec!["HTTP/1.1 200 OK\r\nContent-Length: 51\r\nConnection: close\r\n\r\n{\"access_token\":\"test-token\",\"token_type\":\"Bearer\"}",OK] } else {vec![OK]};
            let (url,worker)=server(responses,move |_,req| {
                let lower = req.to_lowercase();
                if req.contains("/probe") {
                    assert!(lower.contains("x-custom: sentinel-secret-87321"));
                    match auth {
                        "api_key" => assert!(lower.contains("x-test-key: sentinel-secret-87321")),
                        "bearer" | "oauth2" => assert!(lower.contains("authorization: bearer test-token")),
                        "basic" => assert!(req.contains("Basic dXNlcjpwYXNz")),
                        _ => assert!(!lower.contains("authorization:")),
                    }
                }
                assert!(req.contains("/probe")||req.contains("/token"));
                if req.contains("/token") { assert!(req.contains("grant_type=client_credentials")); }
                if req.to_lowercase().contains("x-test-key:") {assert!(req.contains("sentinel-secret-87321"));}
                if req.to_lowercase().contains("authorization: bearer") {assert!(req.contains("test-token"));}
                if req.to_lowercase().contains("authorization: basic") {assert!(req.contains("dXNlcjpwYXNz"));}
            });
            let mut value=config(&url,"api");value["authType"]=json!(auth);value["apiKeyHeader"]=json!("X-Test-Key");value["tokenUrl"]=json!(format!("{url}/token"));value["clientId"]=json!("client");
            let item=save(&conn,"u",&json!({"integration":value,"secrets":{"apiKey":"sentinel-secret-87321","token":"test-token","username":"user","password":"pass","clientSecret":"sentinel-secret-87321","customHeaders":{"X-Custom":"sentinel-secret-87321"}}})).unwrap();
            let id=item["id"].as_str().unwrap();
            let (stored,encrypted)=read(&conn,"u",id).unwrap();
            assert!(stored.has_secrets);
            assert!(!String::from_utf8_lossy(&encrypted).contains("sentinel-secret-87321"));
            assert!(!list(&conn,"u").unwrap().to_string().contains("sentinel-secret-87321"));
            assert_eq!(decode(&encrypted).unwrap().api_key,"sentinel-secret-87321");
            let backup = std::env::temp_dir().join(format!("obr-integrations-{}.sqlite",uuid::Uuid::new_v4()));
            conn.backup(rusqlite::DatabaseName::Main,&backup,None).unwrap();
            let persisted = std::fs::read(&backup).unwrap();
            assert!(!persisted.windows(b"sentinel-secret-87321".len()).any(|w| w==b"sentinel-secret-87321"));
            let reopened = Connection::open(&backup).unwrap();
            assert_eq!(decode(&read(&reopened,"u",id).unwrap().1).unwrap().api_key,"sentinel-secret-87321");
            drop(reopened);std::fs::remove_file(backup).unwrap();
            if auth != "oauth2" {
                let request = Connector::load(&conn,"u",id).unwrap().request().unwrap().build().unwrap();
                assert!(!format!("{:?}",request).contains("sentinel-secret-87321"));
            }
            let result=test_connection(&conn,"u",id).unwrap();
            assert_eq!(result["integration"]["status"],"success");
            assert!(!result.to_string().contains("sentinel-secret-87321"));
            worker.join().unwrap();
        }
    }

    #[test]
    fn official_api_rejects_unsafe_urls_headers_and_redirects() {
        let conn=database();
        for url in ["javascript:alert(1)","file:///C:/x","http://example.com","https://user:pass@example.com","https://example.com/#fragment"] {
            assert!(save(&conn,"u",&json!({"integration":config(url,"official_url")})).is_err());
        }
        let mut api=config("https://example.com","api"); api["endpoint"]=json!("https://other.example/probe");
        assert!(save(&conn,"u",&json!({"integration":api})).is_err());
        api["endpoint"]=json!("/probe");
        assert!(save(&conn,"u",&json!({"integration":api,"secrets":{"customHeaders":{"Host":"evil.example"}}})).is_err());
        let (url,worker)=server(vec!["HTTP/1.1 302 Found\r\nLocation: https://example.invalid/\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"],|_,_|{});
        let item=save(&conn,"u",&json!({"integration":config(&url,"official_url")})).unwrap();
        assert_eq!(test_connection(&conn,"u",item["id"].as_str().unwrap()).unwrap()["integration"]["status"],"failure");worker.join().unwrap();
    }
}

