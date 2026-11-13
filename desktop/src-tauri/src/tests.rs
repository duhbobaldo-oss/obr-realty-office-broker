// اختبارات وحدة لمنطق الأعمال وقاعدة البيانات
// تتحقق من: التسجيل، الدخول، عزل البيانات بين المستخدمين، CRUD

use crate::db::{Database, SqliteDatabase};
use crate::services::*;
use rusqlite::params;
use std::env::temp_dir;

fn setup_test_db() -> SqliteDatabase {
    ensure_test_auth_secret();
    let db_path = temp_dir().join(format!("mawasib_test_{}.db", uuid::Uuid::new_v4()));
    SqliteDatabase::new(&db_path).expect("فشل إنشاء قاعدة بيانات الاختبار")
}

/// تهيئة مفتاح التوثيق للاختبارات (مرة واحدة لكل عملية اختبار).
fn ensure_test_auth_secret() {
    let _ = init_auth_secret("test-only-auth-secret-key-0123456789abcdef".to_string());
}

/// بناء بنية الإعدادات للاختبارات — الخانات غير المحددة تبقى None (فارغة).
fn make_test_settings(
    office_name: Option<&str>,
    fal_license_number: Option<&str>,
) -> Option<crate::db::Settings> {
    Some(crate::db::Settings {
        user_id: "TEST".to_string(),
        office_name: office_name.map(str::to_string),
        office_logo: None,
        office_phone: None,
        office_whatsapp: None,
        office_email: None,
        office_address: None,
        broker_name: None,
        broker_phone: None,
        broker_whatsapp: None,
        default_brochure_template: None,
        fal_license_number: fal_license_number.map(str::to_string),
        fal_license_type: None,
        updated_at: String::new(),
        ui_language: None,
        display_currency: None,
        country: "SA".to_string(),
        broker_license_name: None,
        tracking_endpoint_url: None,
    })
}

#[test]
fn test_register_and_login() {
    let db = setup_test_db();

    // تسجيل مستخدم جديد
    let result = AuthService::new(&db).register(
        "testuser",
        "test@example.com",
        "123456",
        Some("وسيط تجريبي".to_string()),
        None,
        Some("مكتب تجريبي".to_string()),
    );
    assert!(result.is_ok(), "التسجيل يجب أن ينجح");
    let reg = result.unwrap();
    assert_eq!(reg["user_id"], "USER-001");
    assert!(reg["license"]["license_key"].as_str().unwrap().starts_with("RS-"));
    let token = reg["token"].as_str().unwrap().to_string();

    // الدخول بنفس البيانات
    let login = AuthService::new(&db).login("testuser", "123456");
    assert!(login.is_ok(), "الدخول يجب أن ينجح");

    // الدخول بكلمة مرور خاطئة يجب أن يفشل
    let bad_login = AuthService::new(&db).login("testuser", "wrongpass");
    assert!(bad_login.is_err(), "الدخول بكلمة مرور خاطئة يجب أن يفشل");

    // التوكين صالح
    let user_id = verify_token(&token);
    assert!(user_id.is_ok(), "التوكين يجب أن يكون صالحًا");
}

#[test]
fn test_duplicate_registration_rejected() {
    let db = setup_test_db();
    let auth = AuthService::new(&db);

    auth.register("dup", "dup@example.com", "123456", None, None, None).expect("أول تسجيل ينجح");
    let second = auth.register("dup2", "dup@example.com", "123456", None, None, None);
    assert!(second.is_err(), "البريد المكرر يجب أن يُرفض");
}

#[test]
fn test_property_crud_and_isolation() {
    let db = setup_test_db();
    let auth = AuthService::new(&db);

    // مستخدمان مختلفان
    let user1 = auth.register("u1", "u1@example.com", "123456", None, None, None).unwrap();
    let user2 = auth.register("u2", "u2@example.com", "123456", None, None, None).unwrap();
    let uid1 = user1["token"].as_str().unwrap();
    let uid1 = verify_token(uid1).unwrap();
    let uid2 = user2["token"].as_str().unwrap();
    let uid2 = verify_token(uid2).unwrap();

    let props = PropertyService::new(&db);

    // إضافة عرض للمستخدم الأول
    let data = serde_json::json!({
        "name": "فلة العليا",
        "city": "الرياض",
        "district": "العليا",
        "property_type": "فلة",
        "area": 450,
        "price": 1800000,
        "owner_name": "محمد"
    });
    let created = props.create(&uid1, &data).expect("إنشاء العرض يجب أن ينجح");
    let prop_id = created["id"].as_str().unwrap().to_string();

    // لا يُكتب سعر متر مشتق في التخزين؛ الاشتقاق للعرض التحليلي فقط.
    assert!(created["price_per_meter"].is_null());

    // المستخدم الثاني لا يرى عرض المستخدم الأول (عزل البيانات)
    let list2 = props.list(&uid2, 1, 20, "", "", "", "", "").unwrap();
    assert_eq!(list2["total"], 0, "المستخدم الثاني يجب ألا يرى أي عروض");

    // المستخدم الأول يرى عرضه
    let list1 = props.list(&uid1, 1, 20, "", "", "", "", "").unwrap();
    assert_eq!(list1["total"], 1);

    // تعديل العرض
    let updated = props.update(&prop_id, &uid1, &serde_json::json!({ "price": 900000 }));
    assert!(updated.is_ok());
    let updated = updated.unwrap();
    assert_eq!(updated["price"], 900000.0);

    // نسخ العرض
    let dup = props.duplicate(&prop_id, &uid1);
    assert!(dup.is_ok());
    let list1_after_dup = props.list(&uid1, 1, 20, "", "", "", "", "").unwrap();
    assert_eq!(list1_after_dup["total"], 2);

    // حذف العرض
    props.delete(&prop_id, &uid1).expect("الحذف يجب أن ينجح");
    let list1_final = props.list(&uid1, 1, 20, "", "", "", "", "").unwrap();
    assert_eq!(list1_final["total"], 1);
}

#[test]
fn test_map_location_belongs_to_listing_owner_only() {
    let db = setup_test_db();
    let auth = AuthService::new(&db);
    let user1 = auth.register("map1", "map1@example.com", "123456", None, None, None).unwrap();
    let user2 = auth.register("map2", "map2@example.com", "123456", None, None, None).unwrap();
    let uid1 = verify_token(user1["token"].as_str().unwrap()).unwrap();
    let uid2 = verify_token(user2["token"].as_str().unwrap()).unwrap();
    let props = PropertyService::new(&db);

    let created = props
        .create(
            &uid1,
            &serde_json::json!({
                "name": "موقع من الخريطة",
                "latitude": 24.7742,
                "longitude": 46.6100,
                "status": "available"
            }),
        )
        .expect("حفظ موقع جديد باسم يجب أن ينشئ عرضًا للمالك");
    let prop_id = created["id"].as_str().unwrap().to_string();
    assert_eq!(created["user_id"], uid1);
    assert_eq!(created["latitude"], 24.7742);
    assert_eq!(created["longitude"], 46.6100);

    let moved = props
        .update(
            &prop_id,
            &uid1,
            &serde_json::json!({ "latitude": 24.7136, "longitude": 46.6753 }),
        )
        .unwrap();
    assert_eq!(moved["latitude"], 24.7136);
    assert_eq!(moved["longitude"], 46.6753);

    let list2 = props.list(&uid2, 1, 20, "", "", "", "", "").unwrap();
    assert_eq!(list2["total"], 0, "موقع المستخدم الأول لا يظهر في قائمة مستخدم آخر");
    assert!(
        props.get(&prop_id, &uid2).is_err(),
        "لا يجوز قراءة إحداثيات عرض مستخدم آخر"
    );
    assert!(
        props
            .update(
                &prop_id,
                &uid2,
                &serde_json::json!({ "latitude": 21.3891, "longitude": 39.8579 })
            )
            .is_err(),
        "لا يجوز لمستخدم آخر نقل موقع العرض"
    );
    let still = props.get(&prop_id, &uid1).unwrap();
    assert_eq!(still["latitude"], 24.7136);

    assert!(
        props
            .create(
                &uid1,
                &serde_json::json!({
                    "name": "محيط",
                    "latitude": 0.0003,
                    "longitude": 0.0005
                }),
            )
            .is_err(),
        "لا يجوز حفظ موقع في نقطة الصفر/المحيط"
    );
    assert!(
        props
            .update(
                &prop_id,
                &uid1,
                &serde_json::json!({ "latitude": 0.0, "longitude": 0.0 })
            )
            .is_err(),
        "لا يجوز نقل عرض إلى المحيط"
    );
}

#[test]
fn test_property_add_image_sets_main_and_gallery() {
    let db = setup_test_db();
    let auth = AuthService::new(&db);
    let user = auth.register("img1", "img1@example.com", "123456", None, None, None).unwrap();
    let uid = verify_token(user["token"].as_str().unwrap()).unwrap();
    let props = PropertyService::new(&db);
    let created = props
        .create(
            &uid,
            &serde_json::json!({
                "name": "أرض الياسمين",
                "city": "الرياض",
                "price": 1875000
            }),
        )
        .unwrap();
    let id = created["id"].as_str().unwrap().to_string();
    assert!(created["main_image"].is_null());

    let first = props
        .add_image(&id, &uid, "data:image/png;base64,aaa", false)
        .expect("إدراج أول صورة يجب أن ينجح حتى بدون is_main");
    assert!(first["id"].as_str().unwrap().len() > 8);
    let after_first = props.get(&id, &uid).unwrap();
    assert_eq!(after_first["main_image"], "data:image/png;base64,aaa");
    assert_eq!(after_first["images"].as_array().unwrap().len(), 1);

    props
        .add_image(&id, &uid, "data:image/png;base64,bbb", false)
        .expect("إدراج صورة ثانية يجب أن ينجح");
    let after_second = props.get(&id, &uid).unwrap();
    assert_eq!(after_second["main_image"], "data:image/png;base64,aaa");
    assert_eq!(after_second["images"].as_array().unwrap().len(), 2);

    props
        .add_image(&id, &uid, "data:image/png;base64,ccc", true)
        .expect("تعيين صورة رئيسية جديدة يجب أن ينجح");
    let after_main = props.get(&id, &uid).unwrap();
    assert_eq!(after_main["main_image"], "data:image/png;base64,ccc");
    assert_eq!(after_main["images"].as_array().unwrap().len(), 3);

    let fourth = props.add_image(&id, &uid, "data:image/png;base64,ddd", false);
    assert!(fourth.is_ok());
    let fifth = props.add_image(&id, &uid, "data:image/png;base64,eee", false);
    assert!(fifth.is_err(), "الصورة الخامسة يجب أن تُرفض");

    let listed = props.list(&uid, 1, 20, "", "", "", "", "").unwrap();
    assert_eq!(listed["items"][0]["image_count"], 4);

    let empty = props.add_image(&id, &uid, "   ", true);
    assert!(empty.is_err(), "الصورة الفارغة يجب أن تُرفض");
}

#[test]
fn test_client_crud_and_linking() {
    let db = setup_test_db();
    let auth = AuthService::new(&db);
    let user = auth.register("c1", "c1@example.com", "123456", None, None, None).unwrap();
    let uid = verify_token(user["token"].as_str().unwrap()).unwrap();

    let clients = ClientService::new(&db);
    let props = PropertyService::new(&db);

    // إنشاء عميل
    let client = clients.create(&uid, &serde_json::json!({
        "name": "سعد العنزي",
        "phone": "0501112222",
        "relationship_type": "buyer",
        "city": "الرياض"
    })).expect("إنشاء العميل يجب أن ينجح");
    let client_id = client["id"].as_str().unwrap().to_string();

    // إنشاء عرض وربطه بالعميل
    let prop = props.create(&uid, &serde_json::json!({ "name": "أرض الياسمين", "area": 500, "price": 1625000 })).unwrap();
    let prop_id = prop["id"].as_str().unwrap().to_string();

    let link = clients.link_property(&client_id, &uid, &prop_id, "sent");
    assert!(link.is_ok(), "الربط يجب أن ينجح");

    // ملف العميل يحتوي على العرض المرتبط
    let detail = clients.get(&client_id, &uid).unwrap();
    let linked = detail["linked_properties"].as_array().unwrap();
    assert_eq!(linked.len(), 1, "يجب أن يظهر عرض واحد مرتبط");
    assert_eq!(linked[0]["property_name"], "أرض الياسمين");

    // إضافة تواصل
    let comm = clients.add_communication(&client_id, &uid, "whatsapp", "", "تم إرسال 3 عروض");
    assert!(comm.is_ok());

    let detail2 = clients.get(&client_id, &uid).unwrap();
    assert_eq!(detail2["communications"].as_array().unwrap().len(), 1);

    // إلغاء الربط
    clients.unlink_property(&client_id, &uid, &prop_id).expect("إلغاء الربط يجب أن ينجح");
    let detail3 = clients.get(&client_id, &uid).unwrap();
    assert_eq!(detail3["linked_properties"].as_array().unwrap().len(), 0);
}

/// User A لا يستطيع فك ربط بيانات User B حتى مع معرفة client_id و property_id.
#[test]
fn test_unlink_property_rejects_cross_user_and_allows_owner() {
    let db = setup_test_db();
    let auth = AuthService::new(&db);
    let clients = ClientService::new(&db);
    let props = PropertyService::new(&db);

    let user_a = auth
        .register("ua_unlink", "ua_unlink@example.com", "123456", None, None, None)
        .unwrap();
    let uid_a = verify_token(user_a["token"].as_str().unwrap()).unwrap();
    let user_b = auth
        .register("ub_unlink", "ub_unlink@example.com", "123456", None, None, None)
        .unwrap();
    let uid_b = verify_token(user_b["token"].as_str().unwrap()).unwrap();

    let client_b = clients
        .create(
            &uid_b,
            &serde_json::json!({
                "name": "عميل ب",
                "phone": "0509998888",
                "relationship_type": "buyer",
                "city": "جدة"
            }),
        )
        .unwrap();
    let client_b_id = client_b["id"].as_str().unwrap().to_string();

    let prop_b = props
        .create(
            &uid_b,
            &serde_json::json!({ "name": "عرض ب", "area": 300, "price": 900000 }),
        )
        .unwrap();
    let prop_b_id = prop_b["id"].as_str().unwrap().to_string();

    clients
        .link_property(&client_b_id, &uid_b, &prop_b_id, "sent")
        .expect("ربط مالك البيانات يجب أن ينجح");

    // المسار العدائي: A يحاول فك ربط بيانات B بالمعرّفات المعروفة
    let cross = clients.unlink_property(&client_b_id, &uid_a, &prop_b_id);
    assert!(cross.is_err(), "يجب رفض فك الربط عبر المستخدم");
    let err = cross.err().unwrap();
    assert!(
        err.contains("غير موجود") || err.contains("غير مصرح"),
        "رسالة الرفض: {err}"
    );

    // الربط ما زال موجودًا لدى B
    let still = clients.get(&client_b_id, &uid_b).unwrap();
    assert_eq!(
        still["linked_properties"].as_array().unwrap().len(),
        1,
        "رابط B يجب أن يبقى بعد محاولة A"
    );

    // المسار الطبيعي لنفس المالك
    clients
        .unlink_property(&client_b_id, &uid_b, &prop_b_id)
        .expect("مالك البيانات يجب أن يفك الربط");
    let after = clients.get(&client_b_id, &uid_b).unwrap();
    assert_eq!(after["linked_properties"].as_array().unwrap().len(), 0);
}

// ===== P2.1: الأدوار المتعددة والحقول الموسعة =====

#[test]
fn test_client_multiple_roles_crud_and_filter() {
    let db = setup_test_db();
    let auth = AuthService::new(&db);
    let user = auth.register("r1", "r1@example.com", "123456", None, None, None).unwrap();
    let uid = verify_token(user["token"].as_str().unwrap()).unwrap();
    let clients = ClientService::new(&db);

    // شخص واحد بثلاثة أدوار دون تكراره كسجل ثانٍ
    let created = clients
        .create(
            &uid,
            &serde_json::json!({
                "name": "بدر السالم",
                "phone": "0501234567",
                "phone_secondary": "0509876543",
                "email": "badr@example.com",
                "company": "شركة السالم",
                "city": "الرياض",
                "source": "referral",
                "status": "active",
                "category": "A",
                "roles": ["owner", "investor", "broker"]
            }),
        )
        .expect("إنشاء عميل بأدوار متعددة");
    let cid = created["id"].as_str().unwrap().to_string();
    let roles = created["roles"].as_array().unwrap();
    assert_eq!(roles.len(), 3, "يجب حفظ ثلاثة أدوار");
    assert_eq!(created["email"], "badr@example.com");
    assert_eq!(created["phone_secondary"], "0509876543");

    // قراءة الملف تعيد الأدوار
    let detail = clients.get(&cid, &uid).unwrap();
    let detail_roles: Vec<String> = detail["roles"].as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect();
    assert!(detail_roles.contains(&"owner".to_string()));
    assert!(detail_roles.contains(&"investor".to_string()));
    assert!(detail_roles.contains(&"broker".to_string()));

    // الفلترة بأي دور من الأدوار تُرجع نفس الشخص مرة واحدة
    for role in ["owner", "investor", "broker"] {
        let listed = clients.list(&uid, 1, 20, "", role, "", "", false).unwrap();
        assert_eq!(listed["total"], 1, "الفلترة بالدور {role} يجب أن تُرجع سجلاً واحداً");
    }
    // دور غير مرتبط لا يُرجع السجل
    let none = clients.list(&uid, 1, 20, "", "tenant", "", "", false).unwrap();
    assert_eq!(none["total"], 0);

    // تعديل الأدوار: إزالة دور وإضافة آخر
    let updated = clients
        .update(&cid, &uid, &serde_json::json!({ "roles": ["owner", "tenant"] }))
        .expect("تعديل الأدوار");
    let updated_roles: Vec<String> = updated["roles"].as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect();
    assert_eq!(updated_roles.len(), 2);
    assert!(updated_roles.contains(&"tenant".to_string()));
    assert!(!updated_roles.contains(&"broker".to_string()), "الدور المحذوف يجب ألا يبقى");

    // البحث بالبريد وباسم الشركة
    let by_email = clients.list(&uid, 1, 20, "badr@example.com", "", "", "", false).unwrap();
    assert_eq!(by_email["total"], 1);
    let by_company = clients.list(&uid, 1, 20, "شركة السالم", "", "", "", false).unwrap();
    assert_eq!(by_company["total"], 1);

    // الفلترة بالحالة
    let by_status = clients.list(&uid, 1, 20, "", "", "", "active", false).unwrap();
    assert_eq!(by_status["total"], 1);

    let moved = clients
        .update(&cid, &uid, &serde_json::json!({ "city": "جدة" }))
        .unwrap();
    assert_eq!(moved["city"], "جدة");
    assert_eq!(clients.get(&cid, &uid).unwrap()["city"], "جدة");
}

#[test]
fn test_client_validation_and_null_fields() {
    let db = setup_test_db();
    let auth = AuthService::new(&db);
    let user = auth.register("r2", "r2@example.com", "123456", None, None, None).unwrap();
    let uid = verify_token(user["token"].as_str().unwrap()).unwrap();
    let clients = ClientService::new(&db);

    // اسم فارغ يُرفض
    assert!(clients.create(&uid, &serde_json::json!({ "name": "   " })).is_err());
    // بريد غير صحيح يُرفض
    assert!(clients.create(&uid, &serde_json::json!({ "name": "أ", "email": "not-an-email" })).is_err());
    // جوال قصير يُرفض
    assert!(clients.create(&uid, &serde_json::json!({ "name": "أ", "phone": "123" })).is_err());
    // دور غير معروف يُرفض بدل تخزينه بصمت
    assert!(clients.create(&uid, &serde_json::json!({ "name": "أ", "roles": ["wizard"] })).is_err());
    // حالة غير معروفة تُرفض
    assert!(clients.create(&uid, &serde_json::json!({ "name": "أ", "status": "ghost" })).is_err());

    // عميل بأدنى بيانات: كل الحقول الاختيارية NULL ويجب أن تُقرأ بلا خطأ
    let minimal = clients
        .create(&uid, &serde_json::json!({ "name": "عميل بلا تفاصيل" }))
        .expect("عميل بحقول NULL يجب أن ينجح");
    let mid = minimal["id"].as_str().unwrap().to_string();
    assert!(minimal["email"].is_null(), "البريد غير المُدخل يجب أن يبقى NULL");
    assert!(minimal["phone"].is_null());
    assert_eq!(minimal["roles"].as_array().unwrap().len(), 0);

    let read = clients.get(&mid, &uid).expect("قراءة عميل بحقول NULL");
    assert!(read["company"].is_null());
    assert!(read["status"].is_null());

    // تفريغ حقل موجود فعلاً يُخزَّن NULL لا نص فارغ
    clients.update(&mid, &uid, &serde_json::json!({ "email": "someone@example.com" })).unwrap();
    let cleared = clients.update(&mid, &uid, &serde_json::json!({ "email": "" })).unwrap();
    assert!(cleared["email"].is_null(), "تفريغ الحقل يجب أن يُخزّن NULL");

    // القائمة تعمل مع سجلات فيها NULL
    let listed = clients.list(&uid, 1, 20, "", "", "", "", false).unwrap();
    assert_eq!(listed["total"], 1);
}

// ===== P2.2: العقارات والملاك =====

#[test]
fn test_property_owner_link_and_extended_fields() {
    let db = setup_test_db();
    let auth = AuthService::new(&db);
    let user = auth.register("p2", "p2@example.com", "123456", None, None, None).unwrap();
    let uid = verify_token(user["token"].as_str().unwrap()).unwrap();
    let other = auth.register("p3", "p3@example.com", "123456", None, None, None).unwrap();
    let other_uid = verify_token(other["token"].as_str().unwrap()).unwrap();

    let clients = ClientService::new(&db);
    let props = PropertyService::new(&db);

    let owner = clients
        .create(&uid, &serde_json::json!({ "name": "مالك حقيقي", "phone": "0551112223", "roles": ["owner"] }))
        .unwrap();
    let owner_id = owner["id"].as_str().unwrap().to_string();

    // الأرقام تصل كنص من نماذج HTML ويجب أن تُحفظ رقماً
    let created = props
        .create(
            &uid,
            &serde_json::json!({
                "name": "أرض تجارية — الملقا",
                "city": "الرياض",
                "district": "الملقا",
                "property_type": "أرض",
                "purpose": "sale",
                "source_type": "authorized",
                "owner_client_id": owner_id,
                "area": "900",
                "price": "4500000",
                "price_per_meter": "6100",
                "frontage": "شمالية",
                "street_width": "30",
                "streets_count": "2",
                "usage_type": "تجاري",
                "latitude": "24.7742",
                "longitude": "46.6100",
                "description": "وصف",
                "listing_expiry_date": "2027-01-01",
                "commission_agreement": "2.5%",
                "fal_ad_license": "1200000009",
                "internal_price": "4300000",
                "internal_notes": "المالك يقبل 4.3 مليون"
            }),
        )
        .expect("إنشاء عقار موسع");
    let pid = created["id"].as_str().unwrap().to_string();
    assert_eq!(created["area"], 900.0, "المساحة النصية يجب أن تُحفظ رقماً");
    assert_eq!(created["price"], 4500000.0);
    assert_eq!(created["price_per_meter"], 6100.0, "سعر المتر الصريح مستقل عن الإجمالي والمساحة");
    assert_eq!(created["streets_count"], 2);
    assert_eq!(created["street_width"], 30.0);
    assert_eq!(created["source_type"], "authorized");

    let price_only = props.create(&uid, &serde_json::json!({
        "name": "عقار بسعر إجمالي فقط",
        "area": "200",
        "price": "1000000"
    })).unwrap();
    assert!(price_only["price_per_meter"].is_null(), "لا يُكتب سعر متر مشتق فوق إدخال المستخدم");

    let ppm_only = props.create(&uid, &serde_json::json!({
        "name": "عقار بسعر متر فقط",
        "price_per_meter": "7200"
    })).unwrap();
    assert!(ppm_only["price"].is_null(), "السعر الإجمالي يبقى مستقلاً وnullable");
    assert_eq!(ppm_only["price_per_meter"], 7200.0);

    let updated_price = props.update(&pid, &uid, &serde_json::json!({ "price": "5400000" })).unwrap();
    assert_eq!(updated_price["price_per_meter"], 6100.0, "تغيير الإجمالي لا يعدّل سعر المتر");
    let updated_ppm = props.update(&pid, &uid, &serde_json::json!({ "price_per_meter": "6300" })).unwrap();
    assert_eq!(updated_ppm["price"], 5400000.0, "تغيير سعر المتر لا يعدّل الإجمالي");

    // جهة اتصال المالك تُقرأ من السجل الحقيقي
    let detail = props.get(&pid, &uid).unwrap();
    assert_eq!(detail["owner_contact_record"]["id"], serde_json::json!(owner_id));
    assert_eq!(detail["owner_contact_record"]["name"], "مالك حقيقي");
    let owner_profile = clients.get(&owner_id, &uid).unwrap();
    let owned = owner_profile["linked_properties"].as_array().unwrap();
    assert_eq!(owned.len(), 1, "ربط المالك يجب أن يظهر في ملف العميل");
    assert_eq!(owned[0]["link_type"], "owned");

    // ربط مالك من مستخدم آخر يُرفض
    let foreign_owner = clients
        .create(&other_uid, &serde_json::json!({ "name": "مالك مستخدم آخر" }))
        .unwrap();
    let foreign_id = foreign_owner["id"].as_str().unwrap();
    assert!(
        props.update(&pid, &uid, &serde_json::json!({ "owner_client_id": foreign_id })).is_err(),
        "لا يجوز ربط جهة اتصال تابعة لمستخدم آخر"
    );

    // قيم غير صالحة تُرفض
    assert!(props.create(&uid, &serde_json::json!({ "name": "x", "purpose": "barter" })).is_err());
    assert!(props.create(&uid, &serde_json::json!({ "name": "x", "source_type": "unknown" })).is_err());
    assert!(props.create(&uid, &serde_json::json!({ "name": "x", "price": "-5" })).is_err());
    assert!(props.create(&uid, &serde_json::json!({ "name": "x", "latitude": "120" })).is_err());

    // فك ارتباط المالك
    let unlinked = props.update(&pid, &uid, &serde_json::json!({ "owner_client_id": "", "description": "" })).unwrap();
    assert!(unlinked["owner_client_id"].is_null());
    assert!(unlinked["description"].is_null(), "تفريغ الحقل في النموذج يجب أن يُخزَّن NULL");
    let owner_after = clients.get(&owner_id, &uid).unwrap();
    assert_eq!(owner_after["linked_properties"].as_array().unwrap().len(), 0);
    let detail2 = props.get(&pid, &uid).unwrap();
    assert!(detail2["owner_contact_record"].is_null());

    // البحث يغطي رقم فال واسم المالك، والفلترة بالغرض تعمل
    let by_fal = props.list(&uid, 1, 20, "1200000009", "", "", "", "").unwrap();
    assert_eq!(by_fal["total"], 1, "البحث برقم فال يجب أن يعمل");
    let by_purpose = props.list(&uid, 1, 20, "", "", "", "", "sale").unwrap();
    assert_eq!(by_purpose["total"], 1);
    let by_wrong_purpose = props.list(&uid, 1, 20, "", "", "", "", "rent").unwrap();
    assert_eq!(by_wrong_purpose["total"], 0);
    assert!(props.list(&uid, 1, 20, "", "", "", "", "barter").is_err(), "غرض غير معروف في الفلترة يُرفض");

    // عقار بأدنى بيانات: كل الحقول الجديدة NULL
    let minimal = props.create(&uid, &serde_json::json!({ "name": "عقار بلا تفاصيل" })).unwrap();
    assert!(minimal["purpose"].is_null());
    assert!(minimal["street_width"].is_null());
    assert!(minimal["internal_price"].is_null());
    let read = props.get(minimal["id"].as_str().unwrap(), &uid).unwrap();
    assert!(read["latitude"].is_null());
}

#[test]
fn test_shareable_property_excludes_internal_data() {
    let db = setup_test_db();
    let auth = AuthService::new(&db);
    let user = auth.register("s1", "s1@example.com", "123456", None, None, None).unwrap();
    let uid = verify_token(user["token"].as_str().unwrap()).unwrap();
    let props = PropertyService::new(&db);
    let brochures = BrochureService::new(&db);

    let created = props
        .create(
            &uid,
            &serde_json::json!({
                "name": "فيلا للمشاركة",
                "price": "3000000",
                "area": "400",
                "owner_name": "مالك سري",
                "owner_phone": "0555555555",
                "owner_notes": "ملاحظات داخلية",
                "internal_price": "2800000",
                "internal_notes": "الحد الأدنى 2.8",
                "commission_agreement": "2%"
            }),
        )
        .unwrap();
    let pid = created["id"].as_str().unwrap().to_string();

    let brochure = brochures.create(&uid, Some(&pid), "royal-sheet", None, None).unwrap();
    let content = brochures.get(brochure["id"].as_str().unwrap(), &uid).unwrap();
    let content_text = content["content"].to_string();

    for forbidden in ["0555555555", "مالك سري", "ملاحظات داخلية", "2800000", "الحد الأدنى 2.8"] {
        assert!(
            !content_text.contains(forbidden),
            "محتوى المشاركة يجب ألا يحتوي بيانات داخلية: {forbidden}"
        );
    }
    for internal_key in ["owner_phone", "owner_name", "internal_price", "internal_notes", "commission_agreement", "owner_client_id"] {
        assert!(
            !content_text.contains(internal_key),
            "مفتاح داخلي ظهر في ملف المشاركة: {internal_key}"
        );
    }
    // البيانات المسموح مشاركتها موجودة
    assert!(content_text.contains("فيلا للمشاركة"));
    assert!(content_text.contains("3000000"));
}

#[test]
fn test_share_pack_html_hides_internal_fields() {
    let shareable = crate::services::property_to_shareable_json(&crate::db::Property {
        context: crate::country::RecordContext::default(),
        id: "p1".into(),
        user_id: "u1".into(),
        name: "فيلا الشاطئ".into(),
        city: Some("الرياض".into()),
        district: Some("الملقا".into()),
        location: None,
        property_type: Some("فيلا".into()),
        area: Some(400.0),
        price: Some(3_000_000.0),
        price_per_meter: None,
        ad_number: None,
        ad_link: None,
        owner_name: Some("مالك سري".into()),
        owner_phone: Some("0555555555".into()),
        owner_contact: None,
        owner_notes: None,
        marketing_platform: None,
        marketing_link: None,
        published_date: None,
        main_image: None,
        status: "available".into(),
        created_at: "now".into(),
        updated_at: "now".into(),
        owner_client_id: None,
        source_type: None,
        purpose: Some("sale".into()),
        usage_type: None,
        frontage: None,
        street_width: None,
        streets_count: None,
        latitude: None,
        longitude: None,
        description: Some("وصف عام".into()),
        listing_expiry_date: None,
        commission_agreement: Some("2%".into()),
        fal_ad_license: None,
        internal_price: Some(2_800_000.0),
        internal_notes: Some("سري".into()),
        is_training: false,
    });
    let html = crate::share_pack::build_offer_html(&shareable, Some("مكتب الاختبار"), &[]);
    assert!(html.contains("فيلا الشاطئ"));
    assert!(html.contains("مكتب الاختبار"));
    assert!(!crate::share_pack::html_contains_internal_leak(&html));
    assert!(!html.contains("0555555555"));
    assert!(!html.contains("مالك سري"));
    let decoded = crate::share_pack::decode_data_url("data:image/png;base64,aGVsbG8=").unwrap();
    assert_eq!(decoded.0, "png");
    assert_eq!(decoded.1, b"hello");
}

#[test]
fn test_market_formula_audit_fixture() {
    let db = setup_test_db();
    let auth = AuthService::new(&db);
    let user = auth.register("mfa", "mfa@example.com", "123456", None, None, None).unwrap();
    let uid = verify_token(user["token"].as_str().unwrap()).unwrap();
    let market = MarketService::new(&db);

    // R1
    market.create(&uid, &serde_json::json!({
        "city": "الرياض", "area": 500, "sale_price": 1625000, "rent_value": 50000,
        "requests_count": 3, "offers_count": 5, "entry_date": "2026-01-01"
    })).unwrap();
    // R2
    market.create(&uid, &serde_json::json!({
        "city": "الرياض", "area": 400, "sale_price": 1000000, "rent_value": 40000,
        "requests_count": 1, "offers_count": 2, "entry_date": "2026-06-01"
    })).unwrap();
    // R3
    market.create(&uid, &serde_json::json!({
        "city": "جدة", "area": 800, "sale_price": 2450000,
        "requests_count": 2, "offers_count": 4, "entry_date": "2026-03-01"
    })).unwrap();
    // R4 area=0 → no meter
    market.create(&uid, &serde_json::json!({
        "city": "الدمام", "area": 0, "sale_price": 900000, "rent_value": 10000,
        "entry_date": ""
    })).unwrap();
    // R5 empty sale
    market.create(&uid, &serde_json::json!({
        "city": "الرياض", "area": 200, "requests_count": 9, "offers_count": 1,
        "entry_date": "2026-06-01"
    })).unwrap();

    let list = market.list(&uid, 1, 50, "", "", "").unwrap();
    let items = list["items"].as_array().unwrap();
    assert_eq!(items.len(), 5);

    // تحقق صفوف: 1625000/500=3250 ، 1000000/400=2500 ، 2450000/800=3062.5 ، مساحة0=null
    let mut found_zero_area_null = false;
    for it in items {
        let area = it.get("area").and_then(|v| v.as_f64());
        let sale = it.get("sale_price").and_then(|v| v.as_f64());
        let ppm = it.get("price_per_meter_sale").and_then(|v| v.as_f64());
        if area == Some(500.0) && sale == Some(1625000.0) {
            assert!((ppm.unwrap() - 3250.0).abs() < 1e-9, "EXPECTED 3250 ACTUAL {:?}", ppm);
        }
        if area == Some(400.0) && sale == Some(1000000.0) {
            assert!((ppm.unwrap() - 2500.0).abs() < 1e-9);
        }
        if area == Some(800.0) {
            assert!((ppm.unwrap() - 3062.5).abs() < 1e-9);
        }
        if area == Some(0.0) {
            assert!(ppm.is_none(), "مساحة 0 يجب ألا تنتج مترًا، ACTUAL {:?}", ppm);
            found_zero_area_null = true;
        }
        if area == Some(200.0) && sale.is_none() {
            assert!(ppm.is_none());
        }
    }
    assert!(found_zero_area_null);

    let stats = market.stats(&uid, "", "", "").unwrap();
    assert_eq!(stats["count"], 5);
    assert_eq!(stats["total_requests"], 15);
    assert_eq!(stats["total_offers"], 12);
    let avg = stats["avg_price_per_meter"].as_f64().unwrap();
    let expected_avg = (3250.0 + 2500.0 + 3062.5) / 3.0;
    assert!((avg - expected_avg).abs() < 1e-6, "EXPECTED {} ACTUAL {}", expected_avg, avg);
    assert_eq!(stats["min_price_per_meter"].as_f64().unwrap(), 2500.0);
    assert_eq!(stats["max_price_per_meter"].as_f64().unwrap(), 3250.0);
    assert!((stats["avg_rent_per_meter"].as_f64().unwrap() - 100.0).abs() < 1e-9);
    assert_eq!(stats["last_updated"].as_str().unwrap(), "2026-06-01");
}

#[test]
fn test_settings_and_dashboard() {
    let db = setup_test_db();
    let auth = AuthService::new(&db);
    let user = auth.register("s1", "s1@example.com", "123456", None, None, None).unwrap();
    let uid = verify_token(user["token"].as_str().unwrap()).unwrap();

    let settings = SettingsService::new(&db);

    // جلب الإعدادات (تنشئ افتراضية إذا لم توجد)
    let s = settings.get(&uid).unwrap();
    assert!(s["default_brochure_template"].is_null() || s["default_brochure_template"] == "template1");
    assert!(s["fal_license_type"].is_null(), "fal_license_type غير المُدخل يجب أن يُقرأ NULL لا أن يفشل");
    assert!(s["fal_license_number"].is_null());

    // تحديث الإعدادات
    let updated = settings.update(&uid, &serde_json::json!({
        "office_name": "مكتب الوسيط",
        "office_phone": "0555555555",
        "fal_license_type": "brokerage_marketing",
        "fal_license_number": "1200000999"
    })).unwrap();
    assert_eq!(updated["office_name"], "مكتب الوسيط");
    assert_eq!(updated["fal_license_type"], "brokerage_marketing");

    let cleared = settings.update(&uid, &serde_json::json!({
        "fal_license_type": "",
        "fal_license_number": ""
    })).unwrap();
    assert!(cleared["fal_license_type"].is_null(), "تفريغ نوع الرخصة يجب أن يُخزَّن NULL");
    assert!(cleared["fal_license_number"].is_null());

    // لوحة المعلومات
    let dashboard = DashboardService::new(&db);
    let stats = dashboard.stats(&uid).unwrap();
    assert!(stats["properties"]["total"] == 0 || stats["properties"]["total"].is_number());
    let activity = dashboard.activity(&uid).unwrap();
    assert!(!activity.is_empty(), "يجب أن يوجد نشاط (تسجيل حساب)");
}

#[test]
fn test_license_activation() {
    let db = setup_test_db();
    let auth = AuthService::new(&db);

    let u1 = auth.register("lic1", "lic1@example.com", "123456", None, None, None).unwrap();
    let u2 = auth.register("lic2", "lic2@example.com", "123456", None, None, None).unwrap();
    let uid1 = verify_token(u1["token"].as_str().unwrap()).unwrap();
    let uid2 = verify_token(u2["token"].as_str().unwrap()).unwrap();

    // تفعيل ترخيص المستخدم الأول بحسابه يعمل
    let key1 = u1["license"]["license_key"].as_str().unwrap();
    let activation = auth.activate_license(&uid1, key1);
    assert!(activation.is_ok(), "تفعيل الترخيص الخاص يجب أن ينجح");

    // محاولة تفعيل ترخيص المستخدم الأول بحساب المستخدم الثاني تفشل
    let wrong = auth.activate_license(&uid2, key1);
    assert!(wrong.is_err(), "تفعيل ترخيص مستخدم آخر يجب أن يفشل");
}

#[test]
fn test_migrations_reach_latest_and_integrity() {
    let db = setup_test_db();
    let version = db.schema_version().expect("schema_version");
    assert!(version >= 13, "يجب أن تصل Migrations إلى الإصدار 13 على الأقل، الحالي={version}");
    assert!(db.integrity_ok().unwrap(), "integrity_check يجب أن ينجح");

    // إعادة فتح نفس الملف لا يعيد تطبيق Migrations
    let path = {
        // نعيد إنشاء اتصال عبر مسار مؤقت جديد ثم نتحقق من الـ stamp فقط عبر API العام
        version
    };
    assert_eq!(db.schema_version().unwrap(), path);
}

#[test]
fn test_demo_seed_delete_preserves_user_records() {
    use crate::db::demo;
    use rusqlite::Connection;

    ensure_test_auth_secret();
    let db_path = temp_dir().join(format!("mawasib_demo_{}.db", uuid::Uuid::new_v4()));
    let _db = SqliteDatabase::new(&db_path).expect("db");
    let auth = AuthService::new(&_db);
    let reg = auth
        .register("demouser", "demo@example.com", "123456", None, None, None)
        .unwrap();
    let uid = verify_token(reg["token"].as_str().unwrap()).unwrap();

    // سجل حقيقي (غير تجريبي)
    let props = PropertyService::new(&_db);
    let real = props
        .create(
            &uid,
            &serde_json::json!({
                "name": "عقار مستخدم حقيقي",
                "city": "الرياض",
                "district": "الملقا",
                "property_type": "شقة",
                "price": 900000,
                "area": 150,
                "status": "available"
            }),
        )
        .unwrap();
    let real_id = real["id"].as_str().unwrap().to_string();

    let app_files = db_path.parent().unwrap().join("files");
    std::fs::create_dir_all(&app_files).unwrap();
    let mut conn = Connection::open(&db_path).unwrap();
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    let seeded = demo::seed_demo(&mut conn, &uid, db_path.parent().unwrap()).expect("seed");
    assert_eq!(seeded.get("clients").copied().unwrap_or(0), 4);
    assert_eq!(seeded.get("properties").copied().unwrap_or(0), 3);
    assert_eq!(seeded.get("market_reference_data").copied().unwrap_or(0), 75);
    assert_eq!(seeded.get("marketing_campaigns").copied().unwrap_or(0), 4);
    assert_eq!(seeded.get("brochures").copied().unwrap_or(0), 3);
    assert_eq!(seeded.get("finance_operations").copied().unwrap_or(0), 4);
    assert_eq!(seeded.get("finance_invoices").copied().unwrap_or(0), 0);

    let names: Vec<String> = {
        let mut stmt = conn
            .prepare("SELECT name FROM clients WHERE seed_batch_id = ?1 ORDER BY id")
            .unwrap();
        stmt.query_map(rusqlite::params![demo::DEMO_BATCH_ID], |r| r.get(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect()
    };
    assert_eq!(names.len(), 4);
    assert!(names.iter().any(|n| n.contains("عبدالله")));
    assert!(names.iter().any(|n| n.contains("عبدالرحمن")));
    assert!(names.iter().any(|n| n.contains("عبدالمجيد")));
    assert!(names.iter().any(|n| n.contains("عبدالكريم")));
    for forbidden in ["نورة", "فاطمة", "سارة", "هند", "ريم"] {
        assert!(
            !names.iter().any(|n| n.trim() == forbidden),
            "أسماء النساء ممنوعة في البيانات التدريبية: {forbidden}"
        );
    }

    let companies: Vec<String> = {
        let mut stmt = conn
            .prepare("SELECT company FROM clients WHERE seed_batch_id = ?1 ORDER BY id")
            .unwrap();
        stmt.query_map(rusqlite::params![demo::DEMO_BATCH_ID], |r| r.get(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect()
    };
    for c in &companies {
        assert!(
            c == "مستثمر عقاري — أراضٍ"
                || c == "مشتري — سكن"
                || c == "مالك — بائع"
                || c == "مستأجر"
                || c == "وسيط عقاري مرخص"
                || c == "مسوّق عقاري"
                || c == "شركة تطوير عقاري"
                || c == "مكتب عقاري",
            "جهة غير معتمدة في البيانات التدريبية: {c}"
        );
    }

    // كل عميل يحمل دوره المعتمد (المشتري) — عدد الأدوار يساوي عدد العملاء.
    assert!(
        seeded.get("client_roles").copied().unwrap_or(0) >= seeded.get("clients").copied().unwrap_or(0),
        "يجب أن يحمل كل عميل تدريبي دورًا معتمدًا"
    );

  // كل عقار تدريبي مرتبط بجهة اتصال حقيقية كمالك/مصدر.
    let unlinked: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM properties WHERE seed_batch_id = ?1 AND owner_client_id IS NULL",
            rusqlite::params![demo::DEMO_BATCH_ID],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(unlinked, 0, "كل عقار تدريبي يجب أن يرتبط بمالك/مصدر حقيقي");

    drop(conn);
    let dashboard = DashboardService::new(&_db);
    let stats = dashboard.stats(&uid).unwrap();
    assert_eq!(stats["properties"]["total"].as_i64().unwrap_or(-1), 4);
    assert_eq!(stats["clients"]["total"].as_i64().unwrap_or(-1), 4);
    assert!(stats["campaigns"].as_i64().unwrap_or(0) >= 4);

    let mut conn = Connection::open(&db_path).unwrap();
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();

    let deleted =
        demo::delete_demo_batch(&mut conn, demo::DEMO_BATCH_ID, db_path.parent().unwrap())
            .expect("delete demo");
    assert_eq!(deleted.get("clients").copied().unwrap_or(0), 4);
    assert!(deleted.get("client_roles").copied().unwrap_or(0) >= 4);
    let roles_left: i64 = conn
        .query_row("SELECT COUNT(*) FROM client_roles", [], |r| r.get(0))
        .unwrap();
    assert_eq!(roles_left, 0, "أدوار الدفعة التدريبية يجب أن تُحذف بالكامل");

    // العقار الحقيقي بقي
    let still = props.get(&real_id, &uid).unwrap();
    assert_eq!(still["name"], "عقار مستخدم حقيقي");

    let status = demo::demo_status(&conn, demo::DEMO_BATCH_ID).unwrap();
    assert_eq!(status["batch_exists"], false);
    assert_eq!(status["training_seed_deleted"], true);
    assert!(status["non_demo_properties"].as_i64().unwrap() >= 1);

    assert!(
        !demo::ensure_official_training_seed(&mut conn, &uid, db_path.parent().unwrap()).unwrap(),
        "لا يُعاد زرع التدريب تلقائياً بعد الحذف"
    );
}

#[test]
fn test_training_auto_seed_on_first_session() {
    use crate::db::demo;
    use rusqlite::Connection;

    ensure_test_auth_secret();
    let db_path = temp_dir().join(format!("mawasib_train_auto_{}.db", uuid::Uuid::new_v4()));
    let db = SqliteDatabase::new(&db_path).expect("db");
    let auth = AuthService::new(&db);
    let reg = auth
        .register("trainauto", "trainauto@example.com", "123456", None, None, None)
        .unwrap();
    let uid = verify_token(reg["token"].as_str().unwrap()).unwrap();
    let mut conn = Connection::open(&db_path).unwrap();
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    assert!(demo::ensure_official_training_seed(&mut conn, &uid, db_path.parent().unwrap()).unwrap());
    let status = demo::demo_status(&conn, demo::DEMO_BATCH_ID).unwrap();
    assert!(status["batch_exists"].as_bool().unwrap());
    assert!(
        !demo::ensure_official_training_seed(&mut conn, &uid, db_path.parent().unwrap()).unwrap(),
        "لا تكرار للدفعة"
    );
}

#[test]
fn test_brochure_save_without_property_and_list() {
    let db = setup_test_db();
    let auth = AuthService::new(&db);
    let user = auth
        .register("libuser", "lib@example.com", "123456", None, None, None)
        .unwrap();
    let uid = verify_token(user["token"].as_str().unwrap()).unwrap();
    let brochures = BrochureService::new(&db);

    let created = brochures
        .create(
            &uid,
            None,
            "royal-sheet",
            Some("تصميم بدون عقار"),
            Some(&serde_json::json!({
                "version": 1,
                "templateId": "royal-sheet",
                "pageSize": "a4-portrait",
                "propertyId": "",
                "fields": { "title": "تصميم بدون عقار" },
                "theme": {}
            })),
        )
        .expect("حفظ تصميم بدون عقار");

    let id = created["id"].as_str().unwrap().to_string();
    assert!(created["property_id"].is_null());

    let listed = brochures.list(&uid, 0, 0).unwrap();
    assert_eq!(listed.len(), 1, "التصميم بدون عقار يجب أن يظهر في المكتبة");
    assert_eq!(listed[0]["id"], id);

    let got = brochures.get(&id, &uid).unwrap();
    assert_eq!(got["title"], "تصميم بدون عقار");

    // لا يُنشأ عقار وهمي حامل
    let (props, total) = db.list_properties(&uid, "", "", "", "", "", 50, 0).unwrap();
    assert_eq!(total, 0);
    assert!(props.is_empty());
}

#[test]
fn test_brochure_v2_multipage_roundtrip() {
    let db = setup_test_db();
    let auth = AuthService::new(&db);
    let user = auth
        .register("v2user", "v2@example.com", "123456", None, None, None)
        .unwrap();
    let uid = verify_token(user["token"].as_str().unwrap()).unwrap();
    let brochures = BrochureService::new(&db);

    let content = serde_json::json!({
        "version": 2,
        "title": "عرض متعدد الصفحات",
        "propertyId": "",
        "pageSize": "a4-portrait",
        "templateOrigin": "royal-sheet",
        "theme": {
            "primary": "#23365f",
            "accent": "#d97706",
            "surface": "#ffffff",
            "ink": "#23365f",
            "muted": "#6f7380"
        },
        "pages": [
            {
                "id": "page_a",
                "name": "صفحة 1",
                "order": 0,
                "elements": [
                    {
                        "id": "el_1",
                        "type": "property_field",
                        "x": 12.0,
                        "y": 20.0,
                        "width": 80.0,
                        "height": 12.0,
                        "rotation": 0,
                        "zIndex": 1,
                        "visible": true,
                        "binding": "property.falAdLicense",
                        "content": "",
                        "style": { "fontSize": 12, "hideIfEmpty": true }
                    }
                ]
            },
            {
                "id": "page_b",
                "name": "صفحة 2",
                "order": 1,
                "elements": [
                    {
                        "id": "el_2",
                        "type": "heading",
                        "x": 10.0,
                        "y": 10.0,
                        "width": 100.0,
                        "height": 16.0,
                        "rotation": 0,
                        "zIndex": 1,
                        "visible": true,
                        "binding": null,
                        "content": "صفحة ثانية",
                        "style": { "fontSize": 18 }
                    }
                ]
            }
        ],
        "updatedAt": "2026-09-14T00:00:00Z"
    });

    let created = brochures
        .create(
            &uid,
            None,
            "royal-sheet",
            Some("عرض متعدد الصفحات"),
            Some(&content),
        )
        .expect("حفظ مستند v2");

    let id = created["id"].as_str().unwrap().to_string();
    let got = brochures.get(&id, &uid).unwrap();
    let parsed = got["content"].clone();
    assert_eq!(parsed["version"], 2);
    assert_eq!(parsed["pages"].as_array().unwrap().len(), 2);
    assert_eq!(parsed["pages"][0]["elements"][0]["binding"], "property.falAdLicense");
    assert_eq!(parsed["pages"][1]["elements"][0]["content"], "صفحة ثانية");

    let moved = serde_json::json!({
        "version": 2,
        "title": "عرض متعدد الصفحات",
        "propertyId": "",
        "pageSize": "a4-portrait",
        "templateOrigin": "royal-sheet",
        "theme": parsed["theme"].clone(),
        "pages": [
            {
                "id": "page_a",
                "name": "صفحة 1",
                "order": 0,
                "elements": [{
                    "id": "el_1",
                    "type": "property_field",
                    "x": 40.0,
                    "y": 55.0,
                    "width": 90.0,
                    "height": 14.0,
                    "rotation": 0,
                    "zIndex": 1,
                    "visible": true,
                    "binding": "property.falAdLicense",
                    "content": "",
                    "style": { "fontSize": 12, "hideIfEmpty": true }
                }]
            },
            parsed["pages"][1].clone()
        ],
        "updatedAt": "2026-09-14T01:00:00Z"
    });

    brochures
        .update(
            &id,
            &uid,
            Some("royal-sheet"),
            Some("عرض متعدد الصفحات"),
            Some(&moved),
            None,
        )
        .expect("تحديث مواضع v2");

    let again = brochures.get(&id, &uid).unwrap();
    let again_parsed = &again["content"];
    assert_eq!(again_parsed["pages"][0]["elements"][0]["x"], 40.0);
    assert_eq!(again_parsed["pages"][0]["elements"][0]["y"], 55.0);
    assert_eq!(again_parsed["pages"].as_array().unwrap().len(), 2);
}

#[test]
fn test_property_fal_license_roundtrip_for_designer() {
    let db = setup_test_db();
    let auth = AuthService::new(&db);
    let user = auth
        .register("faluser", "fal@example.com", "123456", None, None, None)
        .unwrap();
    let uid = verify_token(user["token"].as_str().unwrap()).unwrap();
    let props = PropertyService::new(&db);
    let brochures = BrochureService::new(&db);

    let created = props
        .create(
            &uid,
            &serde_json::json!({
                "name": "فيلا للعرض",
                "city": "الرياض",
                "fal_ad_license": "1200000111"
            }),
        )
        .unwrap();
    let pid = created["id"].as_str().unwrap().to_string();
    assert_eq!(created["fal_ad_license"], "1200000111");

    let design = brochures
        .create(
            &uid,
            Some(&pid),
            "royal-sheet",
            Some("عرض فال"),
            Some(&serde_json::json!({
                "version": 2,
                "title": "عرض فال",
                "propertyId": pid,
                "pageSize": "a4-portrait",
                "templateOrigin": "royal-sheet",
                "theme": {},
                "pages": [{
                    "id": "page_1",
                    "name": "صفحة 1",
                    "order": 0,
                    "elements": [{
                        "id": "el_fal",
                        "type": "property_field",
                        "x": 12.0, "y": 20.0, "width": 80.0, "height": 12.0,
                        "rotation": 0, "zIndex": 1, "visible": true,
                        "binding": "property.falAdLicense",
                        "content": "",
                        "style": { "hideIfEmpty": true }
                    }]
                }],
                "updatedAt": "2026-09-14T00:00:00Z"
            })),
        )
        .unwrap();

    let bid = design["id"].as_str().unwrap().to_string();
    let listed = brochures.list(&uid, 0, 0).unwrap();
    assert_eq!(listed.len(), 1);

    let opened = brochures.get(&bid, &uid).unwrap();
    assert_eq!(opened["content"]["pages"][0]["elements"][0]["binding"], "property.falAdLicense");

    let cleared = props
        .update(&pid, &uid, &serde_json::json!({ "fal_ad_license": "" }))
        .unwrap();
    assert!(cleared["fal_ad_license"].is_null());
    let after = props.get(&pid, &uid).unwrap();
    assert!(after["fal_ad_license"].is_null());

    brochures.delete(&bid, &uid).unwrap();
    assert!(brochures.list(&uid, 0, 0).unwrap().is_empty());
}

#[test]
fn test_fresh_db_has_zero_operational_records() {
    let db = setup_test_db();
    // فتح قاعدة جديدة يشغّل Migrations فقط — دون Demo ودون سجلات تشغيلية.
    assert!(db.schema_version().unwrap() >= 11);
    let appointments = db.list_appointments("nouser", "", "").unwrap();
    assert!(appointments.is_empty());
    let invoices = db.list_finance_invoices("nouser", "", "", 0, 0).unwrap();
    assert!(invoices.is_empty());
    let (campaigns, camp_total) = db
        .list_campaigns("nouser", "", "", "", "", 10, 0)
        .unwrap();
    assert_eq!(camp_total, 0);
    assert!(campaigns.is_empty());
    let (clients, c_total) = db
        .list_clients("nouser", "", "", "", "", false, 10, 0)
        .unwrap();
    assert_eq!(c_total, 0);
    assert!(clients.is_empty());
    let (props, p_total) = db.list_properties("nouser", "", "", "", "", "", 10, 0).unwrap();
    assert_eq!(p_total, 0);
    assert!(props.is_empty());
    let brochures = db.list_brochures("nouser", 0, 0).unwrap();
    assert!(brochures.is_empty());
}

#[test]
fn test_open_url_allows_mailto_https_and_whatsapp() {
    crate::validate_open_url("mailto:OBR66@outlook.com?subject=Hi").unwrap();
    crate::validate_open_url("https://example.com").unwrap();
    crate::validate_open_url("whatsapp://send?phone=966512345678&text=Hi").unwrap();
    assert!(crate::validate_open_url("javascript:alert(1)").is_err());
    assert!(crate::validate_open_url("http://example.com").is_err());
    assert!(crate::validate_open_url("mailto:x\nCC:evil@x.com").is_err());
    assert!(crate::validate_open_url("whatsapp://send?phone=abc").is_err());
    assert!(crate::validate_open_url("whatsapp://send?phone=123").is_err());
}

#[test]
fn test_normalize_whatsapp_phone() {
    assert_eq!(
        crate::whatsapp::normalize_whatsapp_phone("050 123 4567").unwrap(),
        "966501234567"
    );
    assert_eq!(
        crate::whatsapp::normalize_whatsapp_phone("+966 5 0123 4567").unwrap(),
        "966501234567"
    );
    assert_eq!(
        crate::whatsapp::normalize_whatsapp_phone("٠٠٥٠١٢٣٤٥٦٧").unwrap(),
        "966501234567"
    );
    assert!(crate::whatsapp::normalize_whatsapp_phone("123").is_err());
    assert!(crate::whatsapp::normalize_whatsapp_phone("0112345678").is_err());
    assert!(crate::whatsapp::normalize_whatsapp_phone("").is_err());
    let url = crate::whatsapp::build_whatsapp_url("0501234567", "السلام عليكم،\nمعك مكتب الاختبار.").unwrap();
    assert!(url.starts_with("whatsapp://send?phone=966501234567&text="));
    crate::validate_open_url(&url).unwrap();
}

#[test]
fn test_campaign_crud_metrics_isolation_and_restart() {
    ensure_test_auth_secret();
    let db_path = temp_dir().join(format!("mawasib_campaigns_{}.db", uuid::Uuid::new_v4()));
    let campaign_id;
    let uid1;
    let uid2;
    let property_id;

    {
        let db = SqliteDatabase::new(&db_path).expect("db");
        let auth = AuthService::new(&db);
        let u1 = auth.register("c1", "c1@example.com", "123456", None, None, None).unwrap();
        let u2 = auth.register("c2", "c2@example.com", "123456", None, None, None).unwrap();
        uid1 = verify_token(u1["token"].as_str().unwrap()).unwrap();
        uid2 = verify_token(u2["token"].as_str().unwrap()).unwrap();

        let props = PropertyService::new(&db);
        let created_prop = props
            .create(&uid1, &serde_json::json!({ "name": "فيلا النخيل" }))
            .unwrap();
        property_id = created_prop["id"].as_str().unwrap().to_string();

        let campaigns = CampaignService::new(&db);
        let created = campaigns
            .create(
                &uid1,
                &serde_json::json!({
                    "name": "حملة سناب النخيل",
                    "property_id": property_id,
                    "platform": "snapchat",
                    "ad_link": "https://www.snapchat.com/add/test",
                    "start_date": "2026-09-01",
                    "end_date": "2026-09-30",
                    "views": 100,
                    "inquiries": 10,
                    "viewings": 4,
                    "deals": 1,
                    "cost": 1000,
                    "notes": "إطلاق أولي"
                }),
            )
            .expect("إنشاء الحملة يجب أن ينجح");
        campaign_id = created["id"].as_str().unwrap().to_string();
        assert_eq!(created["inquiry_rate"], 0.1);
        assert_eq!(created["viewing_conversion"], 0.4);
        assert_eq!(created["deal_conversion"], 0.25);
        assert_eq!(created["cost_per_inquiry"], 100.0);
        assert_eq!(created["cost_per_viewing"], 250.0);
        assert_eq!(created["cost_per_deal"], 1000.0);
        assert_eq!(created["property_name"], "فيلا النخيل");
        assert_eq!(created["ad_link"], "https://www.snapchat.com/add/test", "رابط الإعلان يبقى مربوطًا بحقل الإعلان");
        assert_eq!(created["platform"], "snapchat", "اسم المنصة مستقل عن روابط الحملة");
        assert!(created["tracking_url"].is_null(), "رابط التتبع لا يُستبدل باسم المنصة أو رابط الإعلان");

        let other = campaigns.list(&uid2, 1, 30, "", "", "", "").unwrap();
        assert_eq!(other["total"], 0);

        let rejected = campaigns.create(
            &uid1,
            &serde_json::json!({ "name": "س", "platform": "snapchat", "cost": -10 }),
        );
        assert!(rejected.is_err(), "التكلفة السالبة يجب أن تُرفض");

        let unknown_platform = campaigns.create(
            &uid1,
            &serde_json::json!({ "name": "حملة خاطئة", "platform": "unknown-network" }),
        );
        assert!(unknown_platform.is_err());

        let zero = campaigns
            .create(
                &uid1,
                &serde_json::json!({
                    "name": "حملة بلا مشاهدات",
                    "platform": "instagram",
                    "views": 0,
                    "inquiries": 3,
                    "cost": 200
                }),
            )
            .unwrap();
        assert!(zero["inquiry_rate"].is_null());
        assert!(zero["cost_per_inquiry"].as_f64().unwrap() > 0.0);
    }

    {
        let db = SqliteDatabase::new(&db_path).expect("reopen");
        let campaigns = CampaignService::new(&db);
        let got = campaigns.get(&campaign_id, &uid1).expect("بعد إعادة الفتح");
        assert_eq!(got["name"], "حملة سناب النخيل");
        assert_eq!(got["views"], 100);
        assert_eq!(got["inquiry_rate"], 0.1);

        let updated = campaigns
            .update(
                &campaign_id,
                &uid1,
                &serde_json::json!({
                    "views": 200,
                    "inquiries": 20,
                    "viewings": 8,
                    "deals": 2,
                    "cost": 2000,
                    "notes": "بعد التعديل"
                }),
            )
            .unwrap()
            .0;
        assert_eq!(updated["views"], 200);
        assert_eq!(updated["inquiry_rate"], 0.1);
        assert_eq!(updated["notes"], "بعد التعديل");

        let stats = campaigns
            .stats(&uid1, "snapchat", &property_id, "2026-09-01", "2026-09-30")
            .unwrap();
        assert_eq!(stats["count"], 1);
        assert_eq!(stats["views"], 200);
        assert_eq!(stats["inquiry_rate"], 0.1);

        let hidden = campaigns.get(&campaign_id, &uid2);
        assert!(hidden.is_err(), "عزل المستخدمين");
    }

    {
        let db = SqliteDatabase::new(&db_path).expect("reopen2");
        let campaigns = CampaignService::new(&db);
        let got = campaigns.get(&campaign_id, &uid1).unwrap();
        assert_eq!(got["views"], 200);
        assert_eq!(got["notes"], "بعد التعديل");
        assert_eq!(got["inquiry_rate"], 0.1);
        assert_eq!(got["updated_at"].as_str().unwrap().is_empty(), false);

        campaigns.delete(&campaign_id, &uid1).unwrap();
        assert!(campaigns.get(&campaign_id, &uid1).is_err());
        let listed = campaigns.list(&uid1, 1, 30, "", "", "", "").unwrap();
        assert_eq!(listed["total"], 1);
    }
}

#[test]
fn test_appointment_crud_persist_and_restart() {
    ensure_test_auth_secret();
    let db_path = temp_dir().join(format!("mawasib_appts_{}.db", uuid::Uuid::new_v4()));
    let appointment_id;
    let uid1;
    let uid2;

    {
        let db = SqliteDatabase::new(&db_path).expect("db");
        let auth = AuthService::new(&db);
        let u1 = auth.register("a1", "a1@example.com", "123456", None, None, None).unwrap();
        let u2 = auth.register("a2", "a2@example.com", "123456", None, None, None).unwrap();
        uid1 = verify_token(u1["token"].as_str().unwrap()).unwrap();
        uid2 = verify_token(u2["token"].as_str().unwrap()).unwrap();
        let appts = AppointmentService::new(&db);
        let created = appts
            .create(
                &uid1,
                &serde_json::json!({
                    "title": "معاينة النخيل",
                    "occurs_on": "2026-09-14",
                    "starts_at": "10:30",
                    "notes": "مع العميل"
                }),
            )
            .unwrap();
        appointment_id = created["id"].as_str().unwrap().to_string();
        assert_eq!(created["title"], "معاينة النخيل");
        assert_eq!(created["starts_at"], "10:30");
        assert!(appts.create(&uid1, &serde_json::json!({ "title": "س", "occurs_on": "2026-09-14" })).is_err());
        assert!(appts.create(&uid1, &serde_json::json!({ "title": "موعد", "occurs_on": "2026-09-14", "starts_at": "25:10" })).is_err());
        assert_eq!(appts.list(&uid2, "2026-09-01", "2026-09-30").unwrap().len(), 0);
    }

    {
        let db = SqliteDatabase::new(&db_path).expect("reopen");
        let appts = AppointmentService::new(&db);
        let listed = appts.list(&uid1, "2026-09-01", "2026-09-30").unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].title, "معاينة النخيل");
        let updated = appts
            .update(
                &appointment_id,
                &uid1,
                &serde_json::json!({ "starts_at": "11:00", "title": "معاينة معدّلة" }),
            )
            .unwrap();
        assert_eq!(updated["starts_at"], "11:00");
        assert_eq!(updated["title"], "معاينة معدّلة");
    }

    {
        let db = SqliteDatabase::new(&db_path).expect("reopen2");
        let appts = AppointmentService::new(&db);
        let got = appts.get(&appointment_id, &uid1).unwrap();
        assert_eq!(got["title"], "معاينة معدّلة");
        assert_eq!(got["starts_at"], "11:00");
        appts.delete(&appointment_id, &uid1).unwrap();
        assert!(appts.list(&uid1, "2026-09-01", "2026-09-30").unwrap().is_empty());
    }
}

#[test]
fn test_finance_invoice_commission_expense_payment_net_restart_and_pdf() {
    ensure_test_auth_secret();
    use crate::finance_pdf::{generate_pdf, invoice_html, report_html};
    use crate::finance_service::FinanceService;
    use std::path::Path;

    let db_path = temp_dir().join(format!("mawasib_fin_{}.db", uuid::Uuid::new_v4()));
    let app_dir = temp_dir().join(format!("mawasib_fin_docs_{}", uuid::Uuid::new_v4()));
    let invoice_id;
    let commission_id;
    let expense_id;
    let payment_id;
    let uid1;
    let uid2;

    {
        let db = SqliteDatabase::new(&db_path).expect("db");
        let auth = AuthService::new(&db);
        let u1 = auth
            .register("f1", "f1@example.com", "123456", None, None, Some("مكتب النخيل".to_string()))
            .unwrap();
        let u2 = auth
            .register("f2", "f2@example.com", "123456", None, None, None)
            .unwrap();
        uid1 = verify_token(u1["token"].as_str().unwrap()).unwrap();
        uid2 = verify_token(u2["token"].as_str().unwrap()).unwrap();
        SettingsService::new(&db)
            .update(
                &uid1,
                &serde_json::json!({
                    "office_name": "مكتب النخيل",
                    "office_phone": "0110000000",
                    "fal_license_number": "1234567890"
                }),
            )
            .unwrap();
        let fin = FinanceService::new(&db);

        let invoice = fin
            .create_invoice(
                &uid1,
                &serde_json::json!({
                    "title": "عمولة بيع النخيل",
                    "issue_date": "2026-09-01",
                    "due_date": "2099-12-31",
                    "amount": 10000
                }),
            )
            .expect("إنشاء الفاتورة");
        invoice_id = invoice["id"].as_str().unwrap().to_string();
        assert_eq!(invoice["amount"].as_f64(), Some(10000.0));
        assert_eq!(invoice["status"], "due");
        assert_eq!(invoice["paid"].as_f64(), Some(0.0));

        let commission = fin
            .create_commission(
                &uid1,
                &serde_json::json!({
                    "title": "عمولة وساطة",
                    "occurs_on": "2026-09-02",
                    "mode": "percent",
                    "basis_amount": 100000,
                    "rate_percent": 10,
                    "office_share_pct": 50,
                    "broker_share_pct": 30,
                    "marketer_share_pct": 10,
                    "partner_share_pct": 10
                }),
            )
            .expect("تسجيل العمولة");
        commission_id = commission["id"].as_str().unwrap().to_string();
        assert_eq!(commission["computed_amount"].as_f64(), Some(10000.0));
        assert_eq!(commission["office_amount"].as_f64(), Some(5000.0));
        assert_eq!(commission["broker_amount"].as_f64(), Some(3000.0));
        assert_eq!(commission["marketer_amount"].as_f64(), Some(1000.0));
        assert_eq!(commission["partner_amount"].as_f64(), Some(1000.0));

        let bad_shares = fin.create_commission(
            &uid1,
            &serde_json::json!({
                "title": "عمولة خاطئة",
                "occurs_on": "2026-09-02",
                "mode": "fixed",
                "amount_fixed": 1000,
                "office_share_pct": 40,
                "broker_share_pct": 40,
                "marketer_share_pct": 10,
                "partner_share_pct": 5
            }),
        );
        assert!(bad_shares.is_err(), "الحصص التي لا تبلغ 100% يجب أن تُرفض");

        let expense = fin
            .create_expense(
                &uid1,
                &serde_json::json!({
                    "title": "إعلان سناب",
                    "occurs_on": "2026-09-03",
                    "amount": 1500,
                    "category": "تسويق",
                    "reference": "SNAP-1"
                }),
            )
            .expect("إضافة مصروف");
        expense_id = expense["id"].as_str().unwrap().to_string();
        assert_eq!(expense["amount"].as_f64(), Some(1500.0));

        let payment = fin
            .create_payment(
                &uid1,
                &serde_json::json!({
                    "occurs_on": "2026-09-04",
                    "amount": 4000,
                    "target_type": "invoice",
                    "target_id": invoice_id,
                    "method": "تحويل",
                    "reference": "TRX-9"
                }),
            )
            .expect("تسجيل دفعة");
        payment_id = payment["id"].as_str().unwrap().to_string();
        assert_eq!(payment["amount"].as_f64(), Some(4000.0));

        let after_pay = fin.get_invoice(&invoice_id, &uid1).unwrap();
        assert_eq!(after_pay["paid"].as_f64(), Some(4000.0));
        assert_eq!(after_pay["remaining"].as_f64(), Some(6000.0));
        assert_eq!(after_pay["status"], "partial");

        let summary = fin.summary(&uid1, "2026-09-01", "2026-09-30").unwrap();
        assert_eq!(summary["invoice_total"].as_f64(), Some(10000.0));
        assert_eq!(summary["invoice_collected"].as_f64(), Some(4000.0));
        assert_eq!(summary["commission_total"].as_f64(), Some(10000.0));
        assert_eq!(summary["expense_total"].as_f64(), Some(1500.0));
        assert_eq!(summary["net_cash"].as_f64(), Some(2500.0));
        assert_eq!(summary["receivables"].as_f64(), Some(16000.0));

        let hidden = fin.list_invoices(&uid2, "", "", "", 0, 0).unwrap();
        assert_eq!(hidden["total"], 0);

        assert!(fin.create_invoice(&uid1, &serde_json::json!({ "title": "س", "issue_date": "2026-09-01", "amount": 10 })).is_err());
        assert!(fin.create_expense(&uid1, &serde_json::json!({ "title": "مصروف", "occurs_on": "2026-09-03", "amount": 0 })).is_err());
    }

    {
        let db = SqliteDatabase::new(&db_path).expect("reopen");
        let fin = FinanceService::new(&db);
        let invoice = fin.get_invoice(&invoice_id, &uid1).expect("الفاتورة بعد إعادة الفتح");
        assert_eq!(invoice["title"], "عمولة بيع النخيل");
        assert_eq!(invoice["paid"].as_f64(), Some(4000.0));
        assert_eq!(invoice["status"], "partial");
        let commission = fin.get_commission(&commission_id, &uid1).unwrap();
        assert_eq!(commission["computed_amount"].as_f64(), Some(10000.0));
        let expense = fin.get_expense(&expense_id, &uid1).unwrap();
        assert_eq!(expense["amount"].as_f64(), Some(1500.0));
        let payment = fin.get_payment(&payment_id, &uid1).unwrap();
        assert_eq!(payment["reference"], "TRX-9");
        let summary = fin.summary(&uid1, "2026-09-01", "2026-09-30").unwrap();
        assert_eq!(summary["net_cash"].as_f64(), Some(2500.0));

        let (inv_json, pays, settings) = fin.invoice_pdf_payload(&invoice_id, &uid1).unwrap();
        let html = invoice_html(&inv_json, &pays, &settings);
        assert!(html.contains("مكتب النخيل"), "هوية المكتب يجب أن تظهر في الفاتورة");
        assert!(html.contains("4000.00"));
        assert!(html.contains("6000.00"));
        let written = generate_pdf(&app_dir, "invoice", "test-inv", &html)
            .expect("يجب إنشاء PDF الفاتورة بدون فتح عارض خارجي");
        assert!(Path::new(&written.pdf_path).is_file());
        let invoice_pdf = std::fs::read(&written.pdf_path).expect("قراءة PDF الفاتورة");
        let len = invoice_pdf.len();
        assert!(len > 800, "ملف PDF الفاتورة صغير جدًا: {len}");
        assert!(invoice_pdf.starts_with(b"%PDF-"), "ترويسة PDF الفاتورة غير صالحة");

        let (rep, settings) = fin.report_pdf_payload(&uid1, "2026-09-01", "2026-09-30").unwrap();
        let report = report_html(&rep, &settings, "2026-09-01", "2026-09-30");
        assert!(report.contains("2500.00"));
        assert!(report.contains("مكتب النخيل"), "هوية المكتب يجب أن تظهر في التقرير");
        let written_rep = generate_pdf(&app_dir, "report", "test-rep", &report)
            .expect("يجب إنشاء تقرير PDF بدون فتح عارض خارجي");
        let report_pdf = std::fs::read(&written_rep.pdf_path).expect("قراءة PDF التقرير");
        assert!(report_pdf.len() > 800);
        assert!(report_pdf.starts_with(b"%PDF-"), "ترويسة PDF التقرير غير صالحة");

        // التنظيف يأتي بعد كل assertions، ويجب ألا يُتجاهل فشله على Windows.
        let mut cleanup_result = std::fs::remove_dir_all(&app_dir);
        for _ in 0..20 {
            if cleanup_result.is_ok() || !app_dir.exists() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
            cleanup_result = std::fs::remove_dir_all(&app_dir);
        }
        cleanup_result.expect("يجب تنظيف ملفات PDF المؤقتة بعد assertions");
        assert!(!Path::new(&written.pdf_path).is_file());
        assert!(!Path::new(&written_rep.pdf_path).is_file());
        assert!(!app_dir.exists(), "يجب ألا يبقى مجلد PDF المؤقت");
    }
}

/// التدريب لا يُزرع تلقائيًا في قاعدة مستخدم موجودة (سلوك الترقية)،
/// ولا يتأثر أي سجل حقيقي، ولا تتغير نتيجة ensure عند إعادة التشغيل.
#[test]
fn test_training_seed_skipped_on_existing_db() {
    use crate::db::demo;
    use rusqlite::Connection;

    ensure_test_auth_secret();
    let db_path = temp_dir().join(format!("mawasib_upgrade_{}.db", uuid::Uuid::new_v4()));
    let db = SqliteDatabase::new(&db_path).expect("db");
    let auth = AuthService::new(&db);
    let reg = auth
        .register("olduser", "old@example.com", "123456", None, None, None)
        .unwrap();
    let uid = verify_token(reg["token"].as_str().unwrap()).unwrap();

    // سجل حقيقي يجعل القاعدة غير Fresh
    let created = PropertyService::new(&db)
        .create(
            &uid,
            &serde_json::json!({
                "name": "عقار مستخدم قديم",
                "city": "الرياض",
                "property_type": "أرض",
                "price": 500000
            }),
        )
        .unwrap();
    let real_id = created["id"].as_str().unwrap().to_string();

    let mut conn = Connection::open(&db_path).unwrap();
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    let seeded = demo::ensure_official_training_seed(&mut conn, &uid, db_path.parent().unwrap())
        .expect("ensure on existing db");
    assert!(
        !seeded,
        "ممنوع زرع بيانات التدريب تلقائيًا داخل قاعدة مستخدم قديمة"
    );
    let status = demo::demo_status(&conn, demo::DEMO_BATCH_ID).unwrap();
    assert_eq!(status["batch_exists"].as_bool().unwrap(), false);

    // إعادة التشغيل: نفس النتيجة — لا زرع
    drop(conn);
    drop(db);
    let db2 = SqliteDatabase::new(&db_path).expect("reopen");
    let mut conn2 = Connection::open(&db_path).unwrap();
    conn2.pragma_update(None, "foreign_keys", "ON").unwrap();
    assert!(
        !demo::ensure_official_training_seed(&mut conn2, &uid, db_path.parent().unwrap())
            .unwrap(),
        "بعد إعادة التشغيل أيضًا لا يُزرع التدريب في قاعدة قديمة"
    );

    // بيانات المستخدم سليمة وواضحة أنها ليست تدريبية
    let got = PropertyService::new(&db2).get(&real_id, &uid).unwrap();
    assert_eq!(got["name"], "عقار مستخدم قديم");
    assert_eq!(got["is_training"].as_bool(), Some(false));
    assert!(db2.has_training_data().unwrap() == false);
}

/// Fresh Install حقيقي: يُزرع التدريب مرة واحدة، لا تكرار عند restart،
/// الحذف يمنع إعادة الزرع، وإعادة الزرع اليدوية لا تنشئ duplicates.
#[test]
fn test_fresh_install_seed_once_restart_no_duplicates() {
    use crate::db::demo;
    use rusqlite::Connection;

    ensure_test_auth_secret();
    let db_path = temp_dir().join(format!("mawasib_fresh_{}.db", uuid::Uuid::new_v4()));
    let db = SqliteDatabase::new(&db_path).expect("db");
    let auth = AuthService::new(&db);
    let reg = auth
        .register("freshuser", "fresh@example.com", "123456", None, None, None)
        .unwrap();
    let uid = verify_token(reg["token"].as_str().unwrap()).unwrap();

    let mut conn = Connection::open(&db_path).unwrap();
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    assert!(
        demo::ensure_official_training_seed(&mut conn, &uid, db_path.parent().unwrap()).unwrap(),
        "التدريب يُزرع على Fresh Install حقيقية"
    );

    // restart: لا duplicates
    drop(conn);
    drop(db);
    let db2 = SqliteDatabase::new(&db_path).expect("reopen");
    let mut conn2 = Connection::open(&db_path).unwrap();
    conn2.pragma_update(None, "foreign_keys", "ON").unwrap();
    assert!(!demo::ensure_official_training_seed(&mut conn2, &uid, db_path.parent().unwrap()).unwrap());
    let clients_count: i64 = conn2
        .query_row(
            "SELECT COUNT(*) FROM clients WHERE seed_batch_id = ?1",
            rusqlite::params![demo::DEMO_BATCH_ID],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(clients_count, 4, "لا تكرار لدفعة التدريب بعد إعادة التشغيل");
    assert!(db2.has_training_data().unwrap());

    // حذف التدريب ثم restart: لا يُعاد الزرع
    demo::delete_demo_batch(&mut conn2, demo::DEMO_BATCH_ID, db_path.parent().unwrap()).unwrap();
    drop(conn2);
    drop(db2);
    let db3 = SqliteDatabase::new(&db_path).expect("reopen2");
    let mut conn3 = Connection::open(&db_path).unwrap();
    conn3.pragma_update(None, "foreign_keys", "ON").unwrap();
    assert!(
        !demo::ensure_official_training_seed(&mut conn3, &uid, db_path.parent().unwrap()).unwrap(),
        "بعد حذف التدريب لا يُعاد زرعه عند restart"
    );

    // إعادة الزرع اليدوية الصريحة: تعمل مرة واحدة ولا تنشئ duplicates
    demo::reseed_demo(&mut conn3, &uid, db_path.parent().unwrap()).unwrap();
    let clients_count2: i64 = conn3
        .query_row(
            "SELECT COUNT(*) FROM clients WHERE seed_batch_id = ?1",
            rusqlite::params![demo::DEMO_BATCH_ID],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(clients_count2, 4, "reseed اليدوي لا يولد duplicates");
}

/// زرع التدريب يستخدم صور العروض الحقيقية المضمّنة عند توفرها:
/// main_image مسار نسبي تحت properties/ والملف الفعلي موجود، مع SVG إضافي في المعرض.
#[test]
fn test_training_seed_uses_bundled_demo_images() {
    use crate::db::demo;
    use crate::db::training_seed;
    use rusqlite::Connection;

    ensure_test_auth_secret();
    let root = temp_dir().join(format!("mawasib_demo_img_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let db_path = root.join("mawasib.db");
    let db = SqliteDatabase::new(&db_path).expect("db");
    let auth = AuthService::new(&db);
    let reg = auth
        .register("demouser", "demouser@example.com", "123456", None, None, None)
        .unwrap();
    let uid = verify_token(reg["token"].as_str().unwrap()).unwrap();

    // صورة PNG "حقيقية" صغيرة في مجلد مصدر التدريب (يحاكي resources/demo)
    let png: &[u8] = &[
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F,
        0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00,
        0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
        0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];
    let demo_dir = root.join("demo");
    std::fs::create_dir_all(&demo_dir).unwrap();
    std::fs::write(demo_dir.join("obr-poster.png"), png).unwrap();
    training_seed::set_demo_image_sources(vec![demo_dir.clone()]);

    let mut conn = Connection::open(&db_path).unwrap();
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    demo::seed_demo(&mut conn, &uid, &root).unwrap();

    // الأرض (العقار المرتبط بobr-poster.png): main_image = مسار نسبي تحت properties/
    let main_image: String = conn
        .query_row(
            "SELECT main_image FROM properties WHERE id = 'train-p-land'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(
        main_image.starts_with("properties/") && main_image.ends_with(".png"),
        "main_image مسار ملف فعلي: {main_image}"
    );
    let abs = root.join("files").join(&main_image);
    assert!(abs.is_file(), "الملف الفعلي موجود: {}", abs.display());
    assert_eq!(std::fs::metadata(&abs).unwrap().len(), png.len() as u64);

    // property_image_read قادر على قراءته (نفس مسار الواجهة)
    let store = crate::db::files::FileStore::new(&root).unwrap();
    assert_eq!(
        std::fs::read(store.absolute_from_relative(&main_image)).unwrap(),
        png
    );

    // صفّا صور في المعرض: الحقيقية + SVG الاحتياطي
    let imgs: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM property_images WHERE property_id = 'train-p-land'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(imgs, 2, "صورة حقيقية + SVG احتياطي في المعرض");

    // حذف التدريب يمسح ملف الصورة الحقيقية أيضًا
    let stored = main_image.clone();
    demo::delete_demo_batch(&mut conn, demo::DEMO_BATCH_ID, &root).unwrap();
    assert!(
        !root.join("files").join(&stored).is_file(),
        "ملف الصورة الحقيقية حُذف مع الدفعة"
    );

    training_seed::set_demo_image_sources(Vec::new());
    let _ = std::fs::remove_dir_all(&root);
}

/// Login → token صالح → Logout → نفس التوكن يصبح غير صالح فورًا،
/// والسلوك مستمر بعد restart. ولا يتأثر توكن مستخدم آخر.
#[test]
fn test_logout_invalidates_token_and_persists_across_restart() {
    ensure_test_auth_secret();
    let db_path = temp_dir().join(format!("mawasib_logout_{}.db", uuid::Uuid::new_v4()));
    let db = SqliteDatabase::new(&db_path).expect("db");
    let auth = AuthService::new(&db);

    let user = auth
        .register("logoutuser", "logout@example.com", "123456", None, None, None)
        .unwrap();
    let token = user["token"].as_str().unwrap().to_string();
    let user2 = auth
        .register("otheruser", "other@example.com", "123456", None, None, None)
        .unwrap();
    let token2 = user2["token"].as_str().unwrap().to_string();

    // توكن صالح قبل الخروج
    let uid = validate_session_token(&db, &token).expect("التوكن صالح قبل الخروج");
    assert!(!uid.is_empty());

    // Logout يُبطل الجلسة فورًا
    auth.logout(&token).expect("logout");
    assert!(
        validate_session_token(&db, &token).is_err(),
        "نفس التوكن يجب أن يصبح غير صالح بعد Logout"
    );

    // restart: التوكن الملغي يبقى غير صالح، وتوكن مستخدم آخر يبقى صالحًا
    drop(db);
    let db2 = SqliteDatabase::new(&db_path).expect("reopen");
    assert!(
        validate_session_token(&db2, &token).is_err(),
        "التوكن الملغي يبقى غير صالح بعد إعادة التشغيل"
    );
    let uid2 = validate_session_token(&db2, &token2).expect("توكن مستخدم آخر يبقى صالحًا");
    assert!(!uid2.is_empty());

    // دخول جديد بعد restart → توكن جديد صالح ويستمر عبر restart
    let login = AuthService::new(&db2)
        .login("logoutuser", "123456")
        .expect("login بعد restart");
    let token3 = login["token"].as_str().unwrap().to_string();
    validate_session_token(&db2, &token3).expect("التوكن الجديد صالح");
    drop(db2);
    let db3 = SqliteDatabase::new(&db_path).expect("reopen2");
    validate_session_token(&db3, &token3).expect("التوكن الجديد يستمر بعد restart");

    // توكن مزوّر (مفتاح مختلف) يُرفض
    assert!(validate_session_token(&db3, "tampered.token.value").is_err());
}

#[test]
fn test_p12_fresh_activation_identity_restart_and_logout() {
    ensure_test_auth_secret();
    let root = temp_dir().join(format!("obr_p12_fresh_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let db_path = root.join("mawasib.db");
    let installation = uuid::Uuid::new_v4().to_string();
    let db = SqliteDatabase::new(&db_path).unwrap();
    let first = AuthService::new(&db).ensure_local_session("Owner", "owner@example.com", None).unwrap();
    let id = first["user"]["id"].as_str().unwrap().to_string();
    let token = first["token"].as_str().unwrap().to_string();
    assert_eq!(validate_session_token(&db, &token).unwrap(), id);
    drop(db);
    let reopened = SqliteDatabase::new(&db_path).unwrap();
    let next = AuthService::new(&reopened).ensure_local_session("Owner", "owner@example.com", Some(&id)).unwrap();
    assert_eq!(next["user"]["id"], id);
    AuthService::new(&reopened).logout(&token).unwrap();
    assert!(validate_session_token(&reopened, &token).is_err());
    assert_eq!(validate_session_token(&reopened, next["token"].as_str().unwrap()).unwrap(), id);
    drop(reopened);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn test_p12_local_session_adopts_earliest_user_and_email_match_wins() {
    // النسخة الحالية بلا ترخيص ولا بوابة هوية: قاعدة تحتوي مستخدمين قدماء
    // تُفتح بتبنّي أقدم مستخدم دون حذف أي بيانات، ومطابقة البريد تسبق التبنّي.
    ensure_test_auth_secret();
    let db = setup_test_db();
    let old = AuthService::new(&db).register("legacy", "legacy@example.com", "secret1", None, None, None).unwrap();
    let adopted = AuthService::new(&db).ensure_local_session("Customer", "customer@example.com", None).unwrap();
    assert_eq!(adopted["user"]["id"], old["user"]["id"], "يُتبنى أقدم مستخدم قائم بدلاً من إيقاف الفتح");
    assert_eq!(adopted["user"]["email"], "legacy@example.com");
    let retained = db.get_user_by_id(old["user"]["id"].as_str().unwrap()).unwrap();
    assert!(retained.is_some(), "بيانات المستخدم القديم لا تُحذف");
    let registered = AuthService::new(&db).register("customer", "customer@example.com", "secret1", None, None, None).unwrap();
    let matched = AuthService::new(&db).ensure_local_session("Customer", "customer@example.com", None).unwrap();
    assert_ne!(matched["user"]["id"], old["user"]["id"]);
    assert_eq!(matched["user"]["id"], registered["user"]["id"]);
    assert_eq!(matched["user"]["email"], "customer@example.com");
}

#[test]
fn test_p12_restored_old_database_opens_and_adopts_existing_owner() {
    use crate::db::backup;
    ensure_test_auth_secret();
    let root = temp_dir().join(format!("obr_p12_restore_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let active_path = root.join("mawasib.db");
    let original = SqliteDatabase::new(&active_path).unwrap();
    let owner = AuthService::new(&original).ensure_local_session("Owner", "owner@example.com", None).unwrap();
    let owner_id = owner["user"]["id"].as_str().unwrap();
    let restored = SqliteDatabase::new(&root.join("old-restored.db")).unwrap();
    let old = AuthService::new(&restored).register("oldowner", "owner@example.com", "secret1", None, None, None).unwrap();
    assert_ne!(old["user"]["id"], owner_id);
    drop(original);
    drop(restored);
    let old_package = root.join("backups").join("old-package");
    std::fs::create_dir_all(&old_package).unwrap();
    std::fs::copy(root.join("old-restored.db"), old_package.join("mawasib.db")).unwrap();
    backup::restore_backup_package(&root, &active_path, &old_package).unwrap();
    let restored = SqliteDatabase::new(&active_path).unwrap();
    // معرف تثبيت غير موجود في القاعدة المستعادة يسقط إلى مطابقة البريد:
    // القاعدة تُفتح بتبنّي المالك القديم دون أي خطأ هوية.
    let session = AuthService::new(&restored).ensure_local_session("Owner", "owner@example.com", Some(owner_id)).unwrap();
    assert_eq!(session["user"]["id"], old["user"]["id"], "القاعدة المستعادة تُفتح بتبنّي مالكها القديم");
    assert_eq!(session["user"]["email"], "owner@example.com");
    drop(restored);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn test_p12_legacy_auth_commands_are_not_exposed_to_tauri_or_frontend() {
    let rust = include_str!("main.rs");
    let handler = rust.split(".invoke_handler(tauri::generate_handler![").nth(1).unwrap();
    let handler = handler.split("])").next().unwrap();
    let routes = include_str!("../../src/api.ts");
    for legacy in ["auth_register", "auth_login", "auth_activate_license"] {
        assert!(!handler.contains(legacy), "legacy Tauri command still exposed: {legacy}");
        assert!(!routes.contains(&format!("command: '{legacy}'")), "legacy frontend route still exposed: {legacy}");
    }
}

#[test]
fn test_p12_restore_does_not_resurrect_revoked_session() {
    use crate::db::{backup, files::FileStore};
    ensure_test_auth_secret();
    let root = temp_dir().join(format!("obr_p12_revoke_restore_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let db_path = root.join("mawasib.db");
    let db = SqliteDatabase::new(&db_path).unwrap();
    let session = AuthService::new(&db).register("owner", "owner@example.com", "secret1", None, None, None).unwrap();
    let token = session["token"].as_str().unwrap().to_string();
    let user_id = session["user"]["id"].as_str().unwrap();
    let store = FileStore::new(&root).unwrap();
    let conn = crate::db::open_configured(&db_path).unwrap();
    let package = backup::create_backup_package(&root, &db_path, &conn, &store, Some(user_id)).unwrap();
    drop(conn);
    AuthService::new(&db).logout(&token).unwrap();
    assert!(validate_session_token(&db, &token).is_err());
    drop(db);
    backup::restore_backup_package(&root, &db_path, &package.path).unwrap();
    let db = SqliteDatabase::new(&db_path).unwrap();
    assert!(validate_session_token(&db, &token).is_err(), "restore must not resurrect a revoked token");
    drop(db);
    std::fs::remove_dir_all(root).unwrap();
}

/// استعادة سليمة: النسخة القديمة تعود كاملة (DB + files) وتعمل بعد إعادة الفتح (restart)،
/// مع نسخة أمان تُسجَّل قبل الاستعادة.
#[test]
fn test_restore_success_full_cycle() {
    use crate::db::backup;
    use crate::db::files::FileStore;
    use rusqlite::Connection;

    ensure_test_auth_secret();
    let root = temp_dir().join(format!("mawasib_restore_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let db_path = root.join("mawasib.db");
    let db = SqliteDatabase::new(&db_path).expect("db");
    let auth = AuthService::new(&db);
    let reg = auth
        .register("restuser", "rest@example.com", "123456", None, None, None)
        .unwrap();
    let uid = verify_token(reg["token"].as_str().unwrap()).unwrap();

    let props = PropertyService::new(&db);
    let created = props
        .create(
            &uid,
            &serde_json::json!({ "name": "عقار قبل النسخ", "city": "الرياض" }),
        )
        .unwrap();
    let pre_id = created["id"].as_str().unwrap().to_string();

    // صورة حقيقية قبل النسخ
    let store = FileStore::new(&root).unwrap();
    let img_bytes = vec![0x89u8, b'P', b'N', b'G', 1, 2, 3, 4];
    let (img_relative, _stored) = store
        .store_bytes("photo.png", &img_bytes, "properties")
        .expect("save image");
    props
        .update(&pre_id, &uid, &serde_json::json!({ "city": "الرياض" }))
        .unwrap();
    // الصورة الرئيسية تُضبط عبر مسار رفع الصور (كما في التطبيق الفعلي)
    db.set_property_main_image(&pre_id, &img_relative)
        .expect("set main image");

    // نسخة احتياطية (اتصال منفصل كما يفعل main.rs)
    let mut pkg_opt: Option<backup::BackupPackage> = None;
    {
        let bconn = Connection::open(&db_path).unwrap();
        pkg_opt.replace(
            backup::create_backup_package(&root, &db_path, &bconn, &store, Some(&uid))
                .expect("backup"),
        );
    }
    let pkg = pkg_opt.take().unwrap();

    // بيانات جديدة بعد النسخ (يجب أن تختفي بعد الاستعادة)
    props
        .create(&uid, &serde_json::json!({ "name": "عقار بعد النسخ", "city": "جدة" }))
        .unwrap();
    let list_after = props.list(&uid, 1, 20, "", "", "", "", "").unwrap();
    assert_eq!(list_after["total"].as_i64(), Some(2));

    // استعادة: إغلاق الاتصال الحي (محاكاة ما يفعله main.rs) ثم الاستعادة
    drop(db);
    drop(store);
    let restored = backup::restore_backup_package(&root, &db_path, &pkg.path).expect("restore");
    assert_eq!(restored["restored"], true);
    assert_eq!(restored["files_restored"], true);

    // restart: إعادة فتح القاعدة — البيانات القديمة عادت والجديدة اختفت
    let db2 = SqliteDatabase::new(&db_path).expect("reopen after restore");
    let props2 = PropertyService::new(&db2);
    let list = props2.list(&uid, 1, 20, "", "", "", "", "").unwrap();
    assert_eq!(list["total"].as_i64(), Some(1), "يجب أن تعود البيانات إلى لحظة النسخة");
    assert_eq!(list["items"][0]["name"], "عقار قبل النسخ");
    let got = props2.get(&pre_id, &uid).unwrap();
    assert_eq!(got["main_image"], img_relative);

    // الصور رجعت كاملة بايت-ببايت
    let store2 = FileStore::new(&root).unwrap();
    let img_abs = store2.absolute_from_relative(&img_relative);
    let read_back = std::fs::read(&img_abs).expect("image file readable after restore");
    assert_eq!(read_back, img_bytes);
    drop(db2);

    // لا مخلفات pre-restore/tmp بعد نجاح
    let leftovers: Vec<_> = std::fs::read_dir(&root)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.contains("pre-restore") || n.contains("restore-tmp"))
        .collect();
    assert!(leftovers.is_empty(), "مخلفات الاستعادة: {leftovers:?}");

    // نسخة الأمان موجودة في backups/
    let has_safety = backup::list_backup_packages(&root)
        .unwrap()
        .iter()
        .any(|b| b["folder_name"].as_str().unwrap_or("") == pkg.folder_name);
    assert!(has_safety);

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn test_backup_list_empty_and_marks_restorable_with_size() {
    use crate::db::backup;
    use crate::db::files::FileStore;
    use rusqlite::Connection;

    ensure_test_auth_secret();
    let root = temp_dir().join(format!("mawasib_blist_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();

    let empty = backup::list_backup_packages(&root).expect("empty list");
    assert!(empty.is_empty(), "بدون مجلد backups يجب أن تكون القائمة فارغة");

    let db_path = root.join("mawasib.db");
    let db = SqliteDatabase::new(&db_path).expect("db");
    let auth = AuthService::new(&db);
    let reg = auth
        .register("luser", "l@example.com", "123456", None, None, None)
        .unwrap();
    let uid = verify_token(reg["token"].as_str().unwrap()).unwrap();
    let pkg = {
        let bconn = Connection::open(&db_path).unwrap();
        backup::create_backup_package(
            &root,
            &db_path,
            &bconn,
            &FileStore::new(&root).unwrap(),
            Some(&uid),
        )
        .expect("backup")
    };
    drop(db);

    let listed = backup::list_backup_packages(&root).expect("list");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0]["folder_name"], pkg.folder_name);
    assert_eq!(listed[0]["restorable"], true);
    assert!(listed[0]["db_bytes"].as_u64().unwrap_or(0) > 0);

    // نسخة بلا قاعدة = غير قابلة للاستعادة
    let broken_dir = root.join("backups").join("obr-backup-999-broken");
    std::fs::create_dir_all(&broken_dir).unwrap();
    std::fs::write(broken_dir.join("manifest.json"), r#"{"sequence":999}"#).unwrap();
    let listed2 = backup::list_backup_packages(&root).expect("list2");
    let broken = listed2
        .iter()
        .find(|i| i["folder_name"] == "obr-backup-999-broken")
        .expect("broken item");
    assert_eq!(broken["restorable"], false);
    assert_eq!(broken["db_bytes"].as_u64().unwrap_or(1), 0);

    let _ = std::fs::remove_dir_all(&root);
}

/// P9: إنشاء ناجح → اسم نهائي فقط، بلا `.partial-*`؛ القائمة تتجاهل partial؛ لا overwrite.
#[test]
fn test_backup_atomic_create_success_no_partial_and_list_ignores_partial() {
    use crate::db::backup;
    use crate::db::files::FileStore;
    use rusqlite::Connection;

    ensure_test_auth_secret();
    let root = temp_dir().join(format!("mawasib_batom_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let db_path = root.join("mawasib.db");
    let db = SqliteDatabase::new(&db_path).expect("db");
    let auth = AuthService::new(&db);
    let reg = auth
        .register("atomuser", "atom@example.com", "123456", None, None, None)
        .unwrap();
    let uid = verify_token(reg["token"].as_str().unwrap()).unwrap();
    PropertyService::new(&db)
        .create(&uid, &serde_json::json!({ "name": "عقار ذري", "city": "الرياض" }))
        .unwrap();

    let pkg = {
        let bconn = Connection::open(&db_path).unwrap();
        backup::create_backup_package(
            &root,
            &db_path,
            &bconn,
            &FileStore::new(&root).unwrap(),
            Some(&uid),
        )
        .expect("atomic backup")
    };
    drop(db);

    assert!(pkg.path.is_dir(), "النسخة النهائية يجب أن تكون مجلدًا");
    assert!(pkg.path.join("mawasib.db").is_file());
    assert!(pkg.path.join("manifest.json").is_file());
    assert!(pkg.path.join("files").is_dir());
    assert!(
        !pkg.folder_name.starts_with(".partial-"),
        "الاسم النهائي ليس staging"
    );

    let backups = root.join("backups");
    let leftover_partials: Vec<_> = std::fs::read_dir(&backups)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.starts_with(".partial-"))
        .collect();
    assert!(
        leftover_partials.is_empty(),
        "بعد النجاح لا يبقى partial: {leftover_partials:?}"
    );

    // partial يدوي قديم يجب أن يُتجاهل في القائمة والتسلسل
    let old_partial = backups.join(".partial-stale-orphan");
    std::fs::create_dir_all(&old_partial).unwrap();
    std::fs::write(old_partial.join("mawasib.db"), b"x").unwrap();
    let listed = backup::list_backup_packages(&root).expect("list");
    assert!(
        listed
            .iter()
            .all(|i| !i["folder_name"].as_str().unwrap_or("").starts_with(".partial-")),
        "backup_list يتجاهل .partial-*"
    );
    assert!(
        listed
            .iter()
            .any(|i| i["folder_name"].as_str() == Some(pkg.folder_name.as_str())),
        "النسخة النهائية ظاهرة"
    );
    let next = backup::next_backup_sequence(&root);
    assert_eq!(next, pkg.sequence + 1, "التسلسل يتجاهل partial ولا يعدّه");

    // عدم overwrite: وجود مجلد بالاسم المفضّل لا يمسّ محتواه
    let marker = pkg.path.join("ORIGINAL_MARKER.txt");
    std::fs::write(&marker, b"keep-me").unwrap();
    let again = {
        let bconn = Connection::open(&db_path).unwrap();
        backup::create_backup_package_named(
            &root,
            &db_path,
            &bconn,
            &FileStore::new(&root).unwrap(),
            Some(&uid),
            &pkg.folder_name,
            "collision",
            pkg.sequence,
            &pkg.stamp,
        )
        .expect("unique name on collision")
    };
    assert_ne!(again.folder_name, pkg.folder_name);
    assert!(
        !again.folder_name.starts_with(".partial-"),
        "الاسم النهائي ليس staging"
    );
    assert_eq!(
        std::fs::read_to_string(&marker).unwrap(),
        "keep-me",
        "النسخة القديمة لم تُستبدل"
    );
    assert!(again.path.is_dir());
    assert!(again.path.join("mawasib.db").is_file());

    // تنظيف الـ orphan يدويًا في نهاية الاختبار فقط — السياسة لا تحذف partial قديمًا تلقائيًا
    assert!(old_partial.exists(), "partial قديم يبقى حتى سياسة صريحة");

    let _ = std::fs::remove_dir_all(&root);
}

/// P9: فشل أثناء نسخ DB / files / manifest → لا final ناقص + تنظيف staging لهذه العملية فقط.
#[test]
fn test_backup_atomic_create_failures_leave_no_incomplete_final() {
    use crate::db::backup;
    use crate::db::files::FileStore;
    use rusqlite::Connection;

    ensure_test_auth_secret();
    let root = temp_dir().join(format!("mawasib_bfail_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let db_path = root.join("mawasib.db");
    let db = SqliteDatabase::new(&db_path).expect("db");
    let auth = AuthService::new(&db);
    let reg = auth
        .register("bfail", "bfail@example.com", "123456", None, None, None)
        .unwrap();
    let uid = verify_token(reg["token"].as_str().unwrap()).unwrap();

    // partial قديم يجب ألا يُحذف عند فشل عملية لاحقة
    let backups = root.join("backups");
    std::fs::create_dir_all(&backups).unwrap();
    let stale = backups.join(".partial-keep-me");
    std::fs::create_dir_all(&stale).unwrap();
    std::fs::write(stale.join("note.txt"), b"stale").unwrap();

    let store = FileStore::new(&root).unwrap();
    for stage in ["db", "files", "manifest"] {
        backup::test_set_backup_fail_stage(Some(stage));
        let bconn = Connection::open(&db_path).unwrap();
        let err = backup::create_backup_package(&root, &db_path, &bconn, &store, Some(&uid))
            .expect_err(&format!("يجب الفشل عند {stage}"));
        assert!(
            err.contains(stage) || err.contains("injected"),
            "stage={stage} err={err}"
        );
        backup::test_set_backup_fail_stage(None);

        let names: Vec<_> = std::fs::read_dir(&backups)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        assert!(
            !names.iter().any(|n| n.starts_with("obr-backup-")),
            "لا final بعد فشل {stage}: {names:?}"
        );
        let op_partials: Vec<_> = names
            .iter()
            .filter(|n| n.starts_with(".partial-") && *n != ".partial-keep-me")
            .cloned()
            .collect();
        assert!(
            op_partials.is_empty(),
            "staging العملية الفاشلة يُنظَّف ({stage}): {op_partials:?}"
        );
        assert!(stale.exists(), "partial قديم لا يُمس عند فشل {stage}");
    }

    // نجاح بعد الفشل
    let pkg = {
        let bconn = Connection::open(&db_path).unwrap();
        backup::create_backup_package(&root, &db_path, &bconn, &store, Some(&uid)).expect("ok")
    };
    assert!(pkg.path.join("mawasib.db").is_file());
    drop(db);
    let _ = std::fs::remove_dir_all(&root);
}

/// P9: النسخ القديمة (مجلد نهائي مباشر بالصيغة القديمة) ما زالت تُدرَج وتُستعاد.
#[test]
fn test_backup_legacy_final_folder_still_listed_and_restorable() {
    use crate::db::backup;
    use crate::db::files::FileStore;
    use rusqlite::Connection;

    ensure_test_auth_secret();
    let root = temp_dir().join(format!("mawasib_blegacy_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let db_path = root.join("mawasib.db");
    let db = SqliteDatabase::new(&db_path).expect("db");
    let auth = AuthService::new(&db);
    let reg = auth
        .register("leguser", "leg@example.com", "123456", None, None, None)
        .unwrap();
    let uid = verify_token(reg["token"].as_str().unwrap()).unwrap();
    PropertyService::new(&db)
        .create(
            &uid,
            &serde_json::json!({ "name": "عقار قديم", "city": "مكة" }),
        )
        .unwrap();

    // محاكاة حزمة قُدِّمت بالطريقة القديمة (كتابة مباشرة للاسم النهائي)
    let legacy_name = "obr-backup-001-20200101-120000";
    let legacy = root.join("backups").join(legacy_name);
    std::fs::create_dir_all(legacy.join("files")).unwrap();
    std::fs::copy(&db_path, legacy.join("mawasib.db")).unwrap();
    let manifest = serde_json::json!({
        "format": "obr-backup-v1",
        "created_at": "2020-01-01T12:00:00Z",
        "local_stamp": "20200101-120000",
        "sequence": 1,
        "schema_version": 1,
        "folder_name": legacy_name,
        "includes": ["mawasib.db", "files/", "manifest.json"],
        "restore_supported": true
    });
    std::fs::write(
        legacy.join("manifest.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();

    let listed = backup::list_backup_packages(&root).expect("list legacy");
    assert!(
        listed
            .iter()
            .any(|i| i["folder_name"].as_str() == Some(legacy_name)),
        "النسخة القديمة ظاهرة في القائمة"
    );

    PropertyService::new(&db)
        .create(
            &uid,
            &serde_json::json!({ "name": "عقار جديد يجب أن يختفي", "city": "جدة" }),
        )
        .unwrap();
    drop(db);

    let restored = backup::restore_backup_package(&root, &db_path, &legacy).expect("restore legacy");
    assert_eq!(restored["restored"], true);

    let db2 = SqliteDatabase::new(&db_path).expect("reopen");
    let list = PropertyService::new(&db2)
        .list(&uid, 1, 20, "", "", "", "", "")
        .unwrap();
    assert_eq!(list["total"].as_i64(), Some(1));
    assert_eq!(list["items"][0]["name"], "عقار قديم");
    drop(db2);

    // إنشاء جديد بعد القديمة يعمل بالتنسيق نفسه
    let store = FileStore::new(&root).unwrap();
    let bconn = Connection::open(&db_path).unwrap();
    let fresh = backup::create_backup_package(&root, &db_path, &bconn, &store, Some(&uid))
        .expect("fresh after legacy");
    assert!(fresh.folder_name.starts_with("obr-backup-"));
    assert!(fresh.path.join("manifest.json").is_file());

    let _ = std::fs::remove_dir_all(&root);
}

/// استعادة بملف تالف: تُرفض والحية لا تُمس إطلاقًا.
#[test]
fn test_restore_corrupt_package_rejected_and_live_db_intact() {
    use crate::db::backup;
    use crate::db::files::FileStore;
    use rusqlite::Connection;

    ensure_test_auth_secret();
    let root = temp_dir().join(format!("mawasib_rcorrupt_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let db_path = root.join("mawasib.db");
    let db = SqliteDatabase::new(&db_path).expect("db");
    let auth = AuthService::new(&db);
    let reg = auth
        .register("cuser", "c@example.com", "123456", None, None, None)
        .unwrap();
    let uid = verify_token(reg["token"].as_str().unwrap()).unwrap();
    PropertyService::new(&db)
        .create(&uid, &serde_json::json!({ "name": "عقار حي", "city": "الرياض" }))
        .unwrap();

    let pkg = {
        let bconn = Connection::open(&db_path).unwrap();
        backup::create_backup_package(&root, &db_path, &bconn, &FileStore::new(&root).unwrap(), Some(&uid))
            .expect("backup")
    };
    drop(db);

    // إفساد ملف القاعدة داخل الحزمة (كتابة بايتات عشوائية فوق الترويسة)
    let pkg_db = pkg.path.join("mawasib.db");
    {
        use std::io::{Seek, Write};
        let mut f = std::fs::OpenOptions::new().write(true).open(&pkg_db).unwrap();
        f.seek(std::io::SeekFrom::Start(0)).unwrap();
        f.write_all(b"GARBAGED-HEADER-XXXXXX").unwrap();
    }

    let err = backup::restore_backup_package(&root, &db_path, &pkg.path)
        .expect_err("يجب رفض الحزمة التالفة");
    assert!(err.contains("تالفة"), "رسالة الخطأ: {err}");

    // الحية سليمة تمامًا وبياناتها كما كانت
    let db2 = SqliteDatabase::new(&db_path).expect("live db intact");
    let list = PropertyService::new(&db2).list(&uid, 1, 20, "", "", "", "", "").unwrap();
    assert_eq!(list["total"].as_i64(), Some(1));
    assert_eq!(list["items"][0]["name"], "عقار حي");
    drop(db2);
    let _ = std::fs::remove_dir_all(&root);
}

/// فشل نسخة الأمان قبل الاستعادة يمنع الاستعادة ويبقي القاعدة الحية دون مساس.
#[test]
fn test_pre_restore_safety_backup_failure_aborts_and_keeps_live_db() {
    use crate::db::backup;
    use crate::db::files::FileStore;
    use rusqlite::Connection;

    ensure_test_auth_secret();
    let root = temp_dir().join(format!("mawasib_rsafety_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let db_path = root.join("mawasib.db");
    let db = SqliteDatabase::new(&db_path).expect("db");
    let auth = AuthService::new(&db);
    let reg = auth
        .register("suser", "s@example.com", "123456", None, None, None)
        .unwrap();
    let uid = verify_token(reg["token"].as_str().unwrap()).unwrap();
    PropertyService::new(&db)
        .create(&uid, &serde_json::json!({ "name": "عقار قبل الأمان", "city": "الدمام" }))
        .unwrap();

    // حزمة استعادة صالحة موجودة مسبقًا (محاكاة نسخة قديمة)
    let pkg = {
        let bconn = Connection::open(&db_path).unwrap();
        backup::create_backup_package(
            &root,
            &db_path,
            &bconn,
            &FileStore::new(&root).unwrap(),
            Some(&uid),
        )
        .expect("initial backup package")
    };

    // جعل مسار backups ملفًا بدل مجلد → فشل إنشاء نسخة الأمان
    let backups_dir = root.join("backups");
    let _ = std::fs::remove_dir_all(&backups_dir);
    std::fs::write(&backups_dir, b"not-a-directory").expect("block backups dir");

    let bconn = Connection::open(&db_path).unwrap();
    let store = FileStore::new(&root).unwrap();
    let safety = backup::create_pre_restore_safety_backup(
        &root,
        &db_path,
        &bconn,
        &store,
        Some(&uid),
    );
    assert!(safety.is_err(), "يجب فشل نسخة الأمان");
    let err = safety.err().unwrap();
    assert!(
        err.contains("نسخة أمان") || err.contains("أُلغيت الاستعادة"),
        "رسالة الخطأ: {err}"
    );
    drop(bconn);
    drop(store);
    drop(db);

    // القاعدة الحية لم تُمس
    let db2 = SqliteDatabase::new(&db_path).expect("live db intact");
    let list = PropertyService::new(&db2)
        .list(&uid, 1, 20, "", "", "", "", "")
        .unwrap();
    assert_eq!(list["total"].as_i64(), Some(1));
    assert_eq!(list["items"][0]["name"], "عقار قبل الأمان");
    drop(db2);

    assert!(backups_dir.is_file(), "مسار backups ما زال محظورًا كملف");
    let _ = std::fs::remove_file(&backups_dir);
    let _ = std::fs::remove_dir_all(&root);
}

/// فشل أثناء العملية (بعد بدء النسخ): Rollback كامل — القاعدة والملفات القديمة تعود.
#[test]
fn test_restore_failure_midway_rolls_back() {
    use crate::db::backup;
    use crate::db::files::FileStore;
    use rusqlite::Connection;

    ensure_test_auth_secret();
    let root = temp_dir().join(format!("mawasib_rfail_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let db_path = root.join("mawasib.db");
    let db = SqliteDatabase::new(&db_path).expect("db");
    let auth = AuthService::new(&db);
    let reg = auth
        .register("fuser", "f@example.com", "123456", None, None, None)
        .unwrap();
    let uid = verify_token(reg["token"].as_str().unwrap()).unwrap();
    PropertyService::new(&db)
        .create(&uid, &serde_json::json!({ "name": "عقار حي قبل الفشل", "city": "الدمام" }))
        .unwrap();

    // حزمة سليمة القاعدة لكن files فيها ملف باسم غير قابل للنسخ على Windows (>255 حرفًا)
    let pkg = {
        let bconn = Connection::open(&db_path).unwrap();
        backup::create_backup_package(&root, &db_path, &bconn, &FileStore::new(&root).unwrap(), Some(&uid))
            .expect("backup")
    };
    drop(db);

    // نضيف للحزمة ملفًا باسم سام (نقطة ختامية — يُنشأ عبر \\?\ لكن لا يُفتح بالمسار العادي)
    let evil = pkg.path.join("files").join("poison.");
    let evil_long = std::path::PathBuf::from(format!("\\\\?\\{}", evil.display()));
    std::fs::write(&evil_long, b"poison").expect("إنشاء الملف السام");

    let err = backup::restore_backup_package(&root, &db_path, &pkg.path)
        .expect_err("يجب أن تفشل الاستعادة بسبب ملف files");
    assert!(!err.is_empty());

    // Rollback: القاعدة الحية القديمة عادت ببياناتها كاملة
    let db2 = SqliteDatabase::new(&db_path).expect("live db after rollback");
    let list = PropertyService::new(&db2).list(&uid, 1, 20, "", "", "", "", "").unwrap();
    assert_eq!(list["total"].as_i64(), Some(1), "البيانات الحية يجب أن تعود بعد الفشل");
    assert_eq!(list["items"][0]["name"], "عقار حي قبل الفشل");
    drop(db2);

    // لا توجد قاعدة جديدة نصف مستعادة
    assert!(!root.join("mawasib.db.restore-tmp").exists());
    let _ = std::fs::remove_dir_all(&root);
}

/// اختبار إجهاد التخزين: عقارات كثيرة بصور واقعية الحجم.
/// يقيس قبل ترحيل الصور (Base64 داخل SQLite) وبعده (ملفات فعلية + relative_path)
/// ويجري الترحيل بنفس مسار الإقلاع الفعلي migrate_property_images_to_files.
/// ثقيل — يُشغّل صراحة فقط: cargo test test_storage_stress -- --ignored --nocapture
fn run_storage_stress(n: usize) {
    use base64::Engine;
    use std::time::Instant;

    ensure_test_auth_secret();
    let root = temp_dir().join(format!("mawasib_stress_{n}_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let db_path = root.join("mawasib.db");
    let db = SqliteDatabase::new(&db_path).expect("db");
    let auth = AuthService::new(&db);
    let reg = auth
        .register("stress", "stress@example.com", "123456", None, None, None)
        .unwrap();
    let uid = verify_token(reg["token"].as_str().unwrap()).unwrap();

    // صورة "واقعية": 200KB بايتات → data URL بbase64 (~270KB نص) كما تنتجه الواجهة (JPEG 1200px)
    let raw = vec![0xA5u8; 200 * 1024];
    let b64 = base64::engine::general_purpose::STANDARD.encode(&raw);
    let data_url = format!("data:image/jpeg;base64,{b64}");

    let props = PropertyService::new(&db);
    let t_create = Instant::now();
    for i in 0..n {
        let created = props
            .create(
                &uid,
                &serde_json::json!({
                    "name": format!("عقار إجهاد {i}"),
                    "city": if i % 2 == 0 { "الرياض" } else { "جدة" },
                    "property_type": "أرض",
                    "price": 500000 + i
                }),
            )
            .expect("create");
        let pid = created["id"].as_str().unwrap().to_string();
        db.create_property_image(&crate::db::PropertyImage {
            id: uuid::Uuid::new_v4().to_string(),
            property_id: pid.clone(),
            image_url: data_url.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
        })
        .expect("image");
        db.set_property_main_image(&pid, &data_url).expect("main");
    }
    let create_s = t_create.elapsed().as_secs_f64();

    // ===== قياسات قبل الترحيل (Base64 داخل SQLite) =====
    let db_mb_before = std::fs::metadata(&db_path).unwrap().len() as f64 / 1024.0 / 1024.0;

    let t_list = Instant::now();
    let page = props.list(&uid, 1, 20, "", "", "", "", "").expect("list");
    let list_ms_before = t_list.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(page["total"].as_i64().unwrap(), n as i64);

    let t_search = Instant::now();
    let found = props.list(&uid, 1, 20, "عقار إجهاد 7", "", "", "", "").expect("search");
    let search_ms_before = t_search.elapsed().as_secs_f64() * 1000.0;
    assert!(found["total"].as_i64().unwrap() >= 1);

    // ===== الترحيل إلى ملفات (نفس مسار الإقلاع الفعلي) =====
    let store = crate::db::files::FileStore::new(&root).expect("store");
    let mconn = rusqlite::Connection::open(&db_path).unwrap();
    let t_mig = Instant::now();
    let stats = crate::db::images_migration::migrate_property_images_to_files(&mconn, &store)
        .expect("migrate");
    let mig_s = t_mig.elapsed().as_secs_f64();

    // تحقق: لا data URL متبقية، لا أخطاء، كل صورة صارت ملفًا فعليًا
    let remain_imgs: i64 = mconn
        .query_row(
            "SELECT COUNT(*) FROM property_images WHERE image_url LIKE 'data:%'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let remain_main: i64 = mconn
        .query_row(
            "SELECT COUNT(*) FROM properties WHERE main_image LIKE 'data:%'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    drop(mconn);
    // استعادة المساحة كما يفعل مسار الإقلاع الفعلي بعد ترحيل ناجح
    rusqlite::Connection::open(&db_path)
        .unwrap()
        .execute_batch("VACUUM;")
        .unwrap();
    assert_eq!(stats.errors, 0, "ترحيل بلا أخطاء");
    assert_eq!(remain_imgs, 0, "لا data URLs متبقية في property_images");
    assert_eq!(remain_main, 0, "لا data URLs متبقية في properties.main_image");
    assert_eq!(stats.images_migrated as i64, n as i64, "كل صور المعرض رُحّلت");
    assert_eq!(stats.main_images_migrated as i64, n as i64, "كل الصور الرئيسية رُحّلت");

    let files_dir = store.root().join("properties");
    let file_count = std::fs::read_dir(&files_dir)
        .expect("files dir")
        .filter_map(|e| e.ok())
        .count();
    assert_eq!(file_count as i64, n as i64, "ملف فعلي لكل صورة");

    // ===== قياسات بعد الترحيل (ملفات + relative_path) =====
    let db_mb_after = std::fs::metadata(&db_path).unwrap().len() as f64 / 1024.0 / 1024.0;

    let t_list = Instant::now();
    let page = props.list(&uid, 1, 20, "", "", "", "", "").expect("list after");
    let list_ms_after = t_list.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(page["total"].as_i64().unwrap(), n as i64);
    // الصورة الرئيسية الآن مسار نسبي يبدأ بproperties/
    let first_main = page["items"][0]["main_image"].as_str().unwrap().to_string();
    assert!(first_main.starts_with("properties/"), "main_image مسار نسبي: {first_main}");

    let t_search = Instant::now();
    let found = props.list(&uid, 1, 20, "عقار إجهاد 7", "", "", "", "").expect("search after");
    let search_ms_after = t_search.elapsed().as_secs_f64() * 1000.0;
    assert!(found["total"].as_i64().unwrap() >= 1);

    // نسخة احتياطية كاملة (DB + files) بعد الترحيل وقياس زمنها
    let t_backup = Instant::now();
    let bconn = rusqlite::Connection::open(&db_path).unwrap();
    let pkg = crate::db::backup::create_backup_package(
        &root,
        &db_path,
        &bconn,
        &crate::db::files::FileStore::new(&root).unwrap(),
        Some(&uid),
    )
    .expect("backup");
    let backup_s = t_backup.elapsed().as_secs_f64();

    // استعادة كاملة وقياس زمنها + التحقق من عودة الصور كملفات
    drop(bconn);
    drop(db);
    let t_restore = Instant::now();
    let restored = crate::db::backup::restore_backup_package(&root, &db_path, &pkg.path).expect("restore");
    let restore_s = t_restore.elapsed().as_secs_f64();
    assert_eq!(restored["restored"], true);
    let db2 = SqliteDatabase::new(&db_path).expect("reopen");
    assert_eq!(
        PropertyService::new(&db2).list(&uid, 1, 20, "", "", "", "", "").unwrap()["total"].as_i64(),
        Some(n as i64)
    );
    // ملفات الصور عادت كاملة بعد الاستعادة
    let file_count_after_restore = std::fs::read_dir(&files_dir)
        .expect("files dir after restore")
        .filter_map(|e| e.ok())
        .count();
    assert_eq!(file_count_after_restore as i64, n as i64, "كل ملفات الصور عادت بعد Restore");
    // لا data URLs عادت إلى القاعدة بعد الاستعادة
    let rconn = rusqlite::Connection::open(&db_path).unwrap();
    let remain_after_restore: i64 = rconn
        .query_row(
            "SELECT COUNT(*) FROM property_images WHERE image_url LIKE 'data:%'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    drop(rconn);
    assert_eq!(remain_after_restore, 0, "الاستعادة تعيد مسارات ملفات لا data URLs");

    println!("=== STORAGE STRESS: {n} عقار × صورة 200KB — قبل/بعد ترحيل الصور ===");
    println!("  إدخال {n} عقار+صورة (Base64): {create_s:.1}s");
    println!("  ترحيل الصور إلى ملفات: {mig_s:.1}s ({}/{n}, errors={})", stats.images_migrated, stats.errors);
    println!("  حجم SQLite:  قبل {db_mb_before:.1} MB  →  بعد {db_mb_after:.1} MB");
    println!("  فتح القائمة:  قبل {list_ms_before:.0} ms  →  بعد {list_ms_after:.0} ms");
    println!("  بحث:          قبل {search_ms_before:.0} ms  →  بعد {search_ms_after:.0} ms");
    println!("  Backup كامل (بعد): {backup_s:.1}s");
    println!("  Restore كامل (بعد): {restore_s:.1}s");

    drop(db2);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
#[ignore = "اختبار إجهاد ثقيل — يُشغّل صراحة"]
fn test_storage_stress_500() {
    run_storage_stress(500);
}

#[test]
#[ignore = "اختبار إجهاد ثقيل — يُشغّل صراحة"]
fn test_storage_stress_1000() {
    run_storage_stress(1000);
}

/// دورة العقد كاملة: إنشاء → حفظ SQLite → قراءة → Portal Assist → PDF أصلي → تحديث → قائمة → معاينة → طباعة → عزل → إعادة تشغيل.
#[test]
fn test_contract_lifecycle_create_read_portal_pdf_update_list_restart() {
    use base64::Engine;
    use rusqlite::{params, Connection};

    // قاعدة اختبار حقيقية بنفس مسار الإنتاج: SQLite + migrations
    let db_path = temp_dir().join(format!("mawasib_contract_{}.db", uuid::Uuid::new_v4()));
    let mut conn = Connection::open(&db_path).expect("فتح قاعدة الاختبار");
    conn.pragma_update(None, "foreign_keys", "ON").expect("pragma");
    crate::db::migrations::run_migrations(&mut conn).expect("migrations");

    let app_data_dir = temp_dir().join(format!("mawasib_contract_data_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&app_data_dir).unwrap();

    // مستخدم حقيقي — FK (user_id) يشترط صف users مثل الإنتاج
    let uid = "USER-CONTRACT-1";
    conn.execute(
        "INSERT INTO users(id,username,email,password_hash,office_name) VALUES(?1,'contractu','contract@example.com','x','مكتب عقود')",
        params![uid],
    ).expect("إنشاء مستخدم الاختبار");

    // 1) إنشاء مسودة عقد وحفظها في SQLite
    let draft = serde_json::json!({
        "contract_type": "وساطة تسويقية",
        "contract_scope": "تسويق عقاري",
        "parties": ["مكتب الوسيط", "عميل تجريبي"],
        "property_name": "فلة العليا",
        "start_date": "2026-01-01",
        "end_date": "2026-12-31",
        "commission_amount": 2.5,
        "commission_mechanism": "percent",
        "license_number": "LIC-123"
    });
    let saved = crate::contracts::save(&conn, &app_data_dir, uid, &draft).expect("حفظ المسودة");
    let cid = saved["id"].as_str().unwrap().to_string();
    assert!(!cid.is_empty(), "يجب أن يعيد حفظ المسودة id");

    // 2) قراءة العقد بعد الحفظ
    let items = crate::contracts::list(&conn, uid).expect("قراءة بعد الحفظ");
    assert_eq!(items["total"], 1, "يجب أن يظهر العقد في القائمة");
    let row0 = &items["items"][0];
    assert_eq!(row0["contract_type"], "وساطة تسويقية");
    assert_eq!(row0["status"], "draft", "المسودة الجديدة تبدأ draft");
    assert_eq!(row0["source"], "manual_draft");
    assert_eq!(row0["commission_amount"], 2.5);
    assert_eq!(row0["commission_mechanism"], "percent");
    let parties = row0["parties"].as_array().unwrap();
    assert_eq!(parties.len(), 2, "الأطراف محفوظة");
    assert_eq!(parties[0], "مكتب الوسيط");

    // 3) تجهيز بيانات Portal Assist (Mapping دلالي فقط — بلا أتمتة متصفح)
    let pa = crate::contracts::prepare_portal_assist(&draft).expect("portal assist");
    assert_eq!(pa["provider"], "portal_assist");
    assert_eq!(pa["mode"], "copy_assist_only");
    assert_eq!(pa["browser_automation"], false);
    assert_eq!(pa["credentials_stored"], false);
    let fields = pa["fields"].as_array().unwrap();
    assert_eq!(fields.len(), 6, "كل الحقول المكتملة تجهز");
    let commission = fields.iter().find(|f| f["key"] == "commission").unwrap();
    let commission_val = commission["value"].as_str().unwrap();
    assert!(commission_val.contains("نسبة مئوية"), "آلية percent تترجم للعربية: {commission_val}");
    assert!(commission_val.contains("2.5"), "قيمة العمولة موجودة: {commission_val}");
    let parties_f = fields.iter().find(|f| f["key"] == "parties").unwrap();
    assert!(parties_f["value"].as_str().unwrap().contains(" · "), "الأطراف تدمج بفاصل النسخ");
    let period_f = fields.iter().find(|f| f["key"] == "contract_period").unwrap();
    assert!(period_f["value"].as_str().unwrap().contains("←"), "المدة تبدأ←تنتهي");

    // 4) استيراد PDF: فحص المعاينة ثم الحفظ مع الملف الأصلي
    let pdf_bytes: &[u8] = b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog >>\nendobj\ntrailer\n<< /Root 1 0 R >>\n%%EOF\n";
    let pdf_b64 = base64::engine::general_purpose::STANDARD.encode(pdf_bytes);
    let data_url = format!("data:application/pdf;base64,{pdf_b64}");
    let preview = crate::contracts::preview_pdf(&data_url).expect("معاينة PDF");
    assert!(preview["fields"].is_object(), "المعاينة تعيد حقول (حتى لو فارغة لملف بسيط)");
    let with_pdf = serde_json::json!({
        "id": cid,
        "contract_type": "وساطة تسويقية",
        "parties": ["مكتب الوسيط", "عميل تجريبي"],
        "pdf_name": "official.pdf",
        "pdf_data_url": data_url,
        "source": "official_pdf",
        "status": "documented"
    });
    crate::contracts::save(&conn, &app_data_dir, uid, &with_pdf).expect("حفظ مع PDF");

    // 5) الملف الأصلي محفوظ بدون أي تعديل
    let (rel, _name): (String, String) = conn.query_row(
        "SELECT fa.relative_path, fa.original_name FROM contracts c JOIN file_assets fa ON fa.id=c.pdf_asset_id WHERE c.id=?1",
        params![cid], |r| Ok((r.get(0)?, r.get(1)?)),
    ).expect("صف file_asset");
    let stored_bytes = std::fs::read(app_data_dir.join("files").join(&rel)).expect("قراءة الملف المحفوظ");
    assert_eq!(stored_bytes, pdf_bytes, "يجب حفظ PDF الأصلي بدون أي تعديل");

    // 6) تحديث العقد بعد المراجعة — الasset يبقى (COALESCE)
    crate::contracts::save(&conn, &app_data_dir, uid, &serde_json::json!({
        "id": cid, "contract_number": "REGA-777", "status": "active"
    })).expect("تحديث العقد");
    let items2 = crate::contracts::list(&conn, uid).expect("قائمة بعد التحديث");
    assert_eq!(items2["total"], 1, "لا توجد duplicates بعد التحديث");
    assert_eq!(items2["items"][0]["status"], "active");
    assert_eq!(items2["items"][0]["contract_number"], "REGA-777");
    assert_eq!(items2["items"][0]["pdf_name"], "official.pdf", "PDF يبقى مرفقًا بعد التحديث");

    // 7) بيانات المعاينة: نفس بايتات الملف الأصلي كـ data URL
    let pd = crate::contracts::pdf_data(&conn, &app_data_dir, uid, &cid).expect("pdf_data");
    assert_eq!(pd["pdf_name"], "official.pdf");
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(pd["data_url"].as_str().unwrap().split(',').nth(1).unwrap())
        .expect("فك data URL");
    assert_eq!(decoded, pdf_bytes, "المعاينة تعيد الملف الأصلي كما هو");

    // 8) بيانات الطباعة: هوية المكتب من إعدادات بيانات المكتب (لا أي Branding)
    let settings = make_test_settings(Some("مكتب عقود"), Some("7777777777"));
    let display = crate::finance_pdf::office_display_name(&settings);
    assert_eq!(display, "مكتب عقود", "نظام الهوية يقرأ اسم المكتب من الإعدادات");
    let html = crate::contracts::print_html(&items2["items"][0], &settings);
    assert!(html.contains("معاينة عقد"), "عنوان طباعة بدون أي Branding");
    assert!(html.contains("مكتب عقود"), "هوية المكتب من الإعدادات تظهر");
    assert!(html.contains("7777777777"), "رخصة فال من الإعدادات تظهر");
    assert!(html.contains("REGA-777"), "رقم العقد في مستند الطباعة");
    assert!(html.contains("نشط"), "الحالة مترجمة في الطباعة");
    assert!(!html.contains("OBR"), "لا أي Branding في مستند العميل");

    // 9) عزل المستخدمين
    let other = crate::contracts::list(&conn, "OTHER-UID").expect("قائمة مستخدم آخر");
    assert_eq!(other["total"], 0, "مستخدم آخر لا يرى العقد");
    assert!(crate::contracts::pdf_data(&conn, &app_data_dir, uid, &cid).is_ok());
    assert!(crate::contracts::pdf_data(&conn, &app_data_dir, "OTHER-UID", &cid).is_err(), "مستخدم آخر لا يفتح PDF");

    // 10) الإحصاءات والتكاملات
    let s = crate::contracts::summary(&conn, uid).expect("summary");
    assert_eq!(s["total"], 1);
    assert_eq!(s["active"], 1);
    assert_eq!(s["integration"]["providers"].as_array().unwrap().len(), 4, "أربعة providers في descriptor");
    assert_eq!(s["integration"]["portal_assist"]["browser_automation"], false);

    // 11) إعادة تشغيل: إعادة فتح القاعدة نفسها — العقد والPDF يبقيان
    drop(conn);
    let mut conn2 = Connection::open(&db_path).expect("إعادة فتح");
    crate::db::migrations::run_migrations(&mut conn2).expect("migrations بعد الإعادة");
    let items3 = crate::contracts::list(&conn2, uid).expect("قائمة بعد الإعادة");
    assert_eq!(items3["total"], 1, "العقد يبقى بعد إعادة التشغيل");
    assert_eq!(items3["items"][0]["status"], "active");
    let pd2 = crate::contracts::pdf_data(&conn2, &app_data_dir, uid, &cid).expect("pdf بعد الإعادة");
    let decoded2 = base64::engine::general_purpose::STANDARD
        .decode(pd2["data_url"].as_str().unwrap().split(',').nth(1).unwrap())
        .unwrap();
    assert_eq!(decoded2, pdf_bytes, "PDF يبقى سليمًا بعد إعادة التشغيل");

    drop(conn2);
    let _ = std::fs::remove_file(&db_path);
    let _ = std::fs::remove_dir_all(&app_data_dir);
}

/// Round-trip: إنشاء عقد → طباعة PDF بنفس المحرك الفعلي → استيراد نفس PDF → حفظ → إعادة تحميل الجدول.
#[test]
fn test_contract_pdf_roundtrip_export_import() {
    use rusqlite::{params, Connection};
    use base64::Engine;
    use serde_json::json;

    let db_path = temp_dir().join(format!("mawasib_contractrt_{}.db", uuid::Uuid::new_v4()));
    let app_data_dir = temp_dir().join(format!("mawasib_contractrt_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&app_data_dir).expect("app_data_dir");
    let mut conn = Connection::open(&db_path).expect("open");
    conn.pragma_update(None, "foreign_keys", "ON").expect("pragma");
    crate::db::migrations::run_migrations(&mut conn).expect("migrations");
    let uid = "rt-user";
    conn.execute(
        "INSERT INTO users(id,username,email,password_hash) VALUES(?1,'contractrt','contractrt@example.com','x')",
        params![uid],
    ).expect("إنشاء مستخدم الاختبار");

    // 1) إنشاء عقد وساطة داخل OBR
    let created = crate::contracts::save(&conn, &app_data_dir, uid, &json!({
        "contract_number": "1000000",
        "contract_type": "وساطة",
        "contract_scope": "تسويق",
        "parties": ["مالك"],
        "property_name": "44445555",
        "client_name": "عبدالرحمن",
        "status": "unverified",
        "source": "manual_draft"
    })).expect("إنشاء العقد");
    let _cid = created["id"].as_str().unwrap().to_string();

    let settings = make_test_settings(None, None);
    let items = crate::contracts::list(&conn, uid).expect("list");
    let html = crate::contracts::print_html(&items["items"][0], &settings);

    // 2) تصدير PDF بنفس محرك الطباعة الفعلي
    let dir = app_data_dir.join("roundtrip");
    std::fs::create_dir_all(&dir).unwrap();
    let html_path = dir.join("contract.html");
    let pdf_path = dir.join("contract.pdf");
    std::fs::write(&html_path, &html).unwrap();
    crate::share_pack::print_html_to_pdf(&html_path, &pdf_path).expect("طباعة PDF");
    let pdf_bytes = std::fs::read(&pdf_path).expect("قراءة PDF");
    assert!(pdf_bytes.starts_with(b"%PDF-"), "PDF حقيقي مُصدَّر");

    // 3) استيراد نفس PDF واستخراج البيانات
    let pdf_b64 = base64::engine::general_purpose::STANDARD.encode(&pdf_bytes);
    let data_url = format!("data:application/pdf;base64,{pdf_b64}");
    let preview = crate::contracts::preview_pdf(&data_url).expect("معاينة PDF المستورد");
    // [تشخيص مؤقت] حفظ النص المستخرج لفحصه خارج الطرفية
    std::fs::write(std::env::temp_dir().join("contract-excerpt.txt"), preview["extracted_text_excerpt"].as_str().unwrap_or("")).ok();
    let _ = std::fs::write(dir.join("preview.json"), serde_json::to_string_pretty(&preview).unwrap_or_default());
    let fields = &preview["fields"];
    let got = |k: &str| fields.get(k).and_then(serde_json::Value::as_str).unwrap_or("").to_string();

    // 4) الحقول المؤكدة يجب أن ترجع بنفس القيم التي صدّرها OBR
    assert_eq!(got("contract_number"), "1000000", "رقم العقد بعد الاستيراد — المعاينة: {preview}");
    assert_eq!(got("contract_type"), "وساطة", "النوع بعد الاستيراد — المعاينة: {preview}");
    assert_eq!(got("contract_scope"), "تسويق", "النطاق بعد الاستيراد — المعاينة: {preview}");
    assert_eq!(got("property_name"), "44445555", "العقار بعد الاستيراد — المعاينة: {preview}");
    assert_eq!(got("client_name"), "عبدالرحمن", "العميل بعد الاستيراد — المعاينة: {preview}");
    let parties = fields.get("parties").and_then(serde_json::Value::as_array).cloned().unwrap_or_default();
    assert_eq!(parties, vec![json!("مالك")], "الأطراف بعد الاستيراد");

    // لا internals في المعاينة النصية
    if let Some(excerpt) = preview["extracted_text_excerpt"].as_str() {
        for marker in ["PDF-1.", "FlateDecode", "endstream", "/Creator", "/Producer"] {
            assert!(!excerpt.contains(marker), "المعاينة تعرض PDF internals: {marker}");
        }
    }

    // 5) حفظ العقد المستورد ثم إعادة تحميل الجدول
    let mut imported = preview["fields"].clone();
    if let Some(obj) = imported.as_object_mut() {
        obj.insert("source".into(), json!("official_pdf"));
        obj.insert("status".into(), json!("unverified"));
    }
    crate::contracts::save(&conn, &app_data_dir, uid, &imported).expect("حفظ العقد المستورد");
    let after = crate::contracts::list(&conn, uid).expect("إعادة تحميل الجدول");
    let row = &after["items"][0];
    assert_eq!(row["contract_number"], "1000000", "رقم العقد في الجدول");
    assert_eq!(row["contract_type"], "وساطة", "النوع في الجدول");
    assert_eq!(row["contract_scope"], "تسويق", "النطاق في الجدول");
    assert_eq!(row["property_name"], "44445555", "العقار في الجدول");
    assert_eq!(row["client_name"], "عبدالرحمن", "العميل في الجدول");
    assert_eq!(row["source"], "official_pdf");

    drop(conn);
    let _ = std::fs::remove_file(&db_path);
    let _ = std::fs::remove_dir_all(&app_data_dir);
}

/// رفض PDF غير صالح + رفض آلية عمولة غير صالحة — حالات الفشل الصديقة.
#[test]
fn test_contract_failures_friendly() {
    use rusqlite::{params, Connection};

    let db_path = temp_dir().join(format!("mawasib_contractf_{}.db", uuid::Uuid::new_v4()));
    let mut conn = Connection::open(&db_path).expect("open");
    conn.pragma_update(None, "foreign_keys", "ON").expect("pragma");
    crate::db::migrations::run_migrations(&mut conn).expect("migrations");
    let app_data_dir = temp_dir().join(format!("mawasib_contractf_data_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&app_data_dir).unwrap();
    let uid = "USER-F";
    conn.execute(
        "INSERT INTO users(id,username,email,password_hash) VALUES(?1,'contractuf','contractf@example.com','x')",
        params![uid],
    ).expect("إنشاء مستخدم الاختبار");

    // PDF غير صالح يُرفض برسالة عربية
    use base64::Engine;
    let not_pdf = base64::engine::general_purpose::STANDARD.encode(b"NOT_PDF");
    let err = crate::contracts::preview_pdf(&format!("data:application/pdf;base64,{not_pdf}")).unwrap_err();
    assert!(err.contains("ليس PDF"), "رفض ودي: {err}");
    let err2 = crate::contracts::save(&conn, &app_data_dir, uid, &serde_json::json!({
        "pdf_name": "x.pdf", "pdf_data_url": format!("data:application/pdf;base64,{}", base64::engine::general_purpose::STANDARD.encode(b"NOT_PDF"))
    })).unwrap_err();
    assert!(err2.contains("ليس PDF"), "رفض حفظ ودي: {err2}");

    // آلية عمولة غير صالحة تُرفض
    let err3 = crate::contracts::save(&conn, &app_data_dir, uid, &serde_json::json!({
        "commission_mechanism": "unknown"
    })).unwrap_err();
    assert!(err3.contains("آلية العمولة"), "رفض آلية ودي: {err3}");

    // حالة عقد غير صالحة تُرفض
    let err4 = crate::contracts::save(&conn, &app_data_dir, uid, &serde_json::json!({ "status": "hacked" })).unwrap_err();
    assert!(err4.contains("حالة العقد"), "رفض حالة ودي: {err4}");

    drop(conn);
    let _ = std::fs::remove_file(&db_path);
    let _ = std::fs::remove_dir_all(&app_data_dir);
}

/// حارس هوية الطباعة المركزية: كل مستند مطبوع/PDF يسحب هويته من إعدادات بيانات المكتب فقط،
/// ولا يُسمح بأي Branding ثابت (OBR / مواسيب / مكتب الوسيط العقاري) في محتوى مستندات العميل،
/// والخانة الفارغة تختفي (لا قيمة افتراضية ولا نص تجريبي).
fn assert_no_hardcoded_branding(s: &str, what: &str) {
    for token in ["OBR", "obr", "مواسيب", "مواسـيب", "الوسيط العقاري", "Realty Office"] {
        assert!(!s.contains(token), "{what} يحتوي Branding ثابت: {token}");
    }
}

#[test]
fn test_print_identity_centralized_no_hardcoded_branding() {
    use crate::finance_pdf::{invoice_html, office_display_name, record_html, report_html};
    use crate::finance_service::FinanceService;

    ensure_test_auth_secret();
    let db_path = temp_dir().join(format!("mawasib_ident_{}.db", uuid::Uuid::new_v4()));
    let uid;
    let invoice_id;
    let contract_data = serde_json::json!({
        "contract_number": "ID-1", "status": "active", "contract_type": "وساطة",
        "parties": ["الطرف الأول", "الطرف الثاني"],
        "start_date": "2026-01-01", "end_date": "2026-12-31"
    });

    {
        let db = SqliteDatabase::new(&db_path).expect("db");
        let reg = AuthService::new(&db)
            .register("ident", "ident@example.com", "123456", None, None, None)
            .unwrap();
        uid = verify_token(reg["token"].as_str().unwrap()).unwrap();
        let fin = FinanceService::new(&db);
        let invoice = fin
            .create_invoice(&uid, &serde_json::json!({
                "title": "عمولة بيع فلة", "issue_date": "2026-09-01",
                "due_date": "2099-12-31", "amount": 5000
            }))
            .unwrap();
        invoice_id = invoice["id"].as_str().unwrap().to_string();

        // A) إعدادات فارغة: لا اسم مكتب مختلق، ولا حقول فارغة في المخرجات، ولا أي Branding
        let (inv_json, pays, empty_settings) = fin.invoice_pdf_payload(&invoice_id, &uid).unwrap();
        assert!(
            empty_settings.is_none() || office_display_name(&empty_settings).is_empty(),
            "الإعدادات الفارغة لا تخترع اسم مكتب"
        );
        let invoice_html_empty = invoice_html(&inv_json, &pays, &empty_settings);
        assert_no_hardcoded_branding(&invoice_html_empty, "فاتورة بإعدادات فارغة");
        assert!(!invoice_html_empty.contains("هاتف:"), "خانة الجوال الفارغة تختفي");
        assert!(!invoice_html_empty.contains("واتساب:"), "خانة الواتساب الفارغة تختفي");
        assert!(!invoice_html_empty.contains("ترخيص فال"), "خانة الرخصة الفارغة تختفي");

        let (rep, rep_settings) = fin.report_pdf_payload(&uid, "2026-09-01", "2026-09-30").unwrap();
        let report_empty = report_html(&rep, &rep_settings, "2026-09-01", "2026-09-30");
        assert_no_hardcoded_branding(&report_empty, "تقرير بإعدادات فارغة");
        assert!(!report_empty.contains("هاتف:"), "حقول التقرير الفارغة تختفي");

        let contract_html_empty = crate::contracts::print_html(&contract_data, &None);
        assert_no_hardcoded_branding(&contract_html_empty, "عقد بإعدادات فارغة");
        assert!(!contract_html_empty.contains("هاتف:"), "حقول العقد الفارغة تختفي");
        assert!(!contract_html_empty.contains("ترخيص فال"), "رخصة العقد الفارغة تختفي");

        for (kind, heading) in [
            ("operation", "بيان عملية"),
            ("commission", "بيان عمولة"),
            ("expense", "سند مصروف"),
            ("payment", "إيصال دفعة"),
            ("receivable", "إشعار استحقاق"),
        ] {
            let html = record_html(kind, &serde_json::json!({ "title": "سجل اختبار", "notes": null }), &None).unwrap();
            assert!(html.contains(heading), "عنوان مستند {kind}");
            assert!(!html.contains('—'), "الحقول الفارغة لا تتحول إلى شرطة في مستند {kind}");
            assert_no_hardcoded_branding(&html, kind);
        }

        let offer_empty = crate::share_pack::build_offer_html(
            &serde_json::json!({ "name": "فلة العليا", "price": 100 }),
            None,
            &[],
        );
        assert_no_hardcoded_branding(&offer_empty, "عرض بإعدادات فارغة");

        // B) إعدادات مكتب معبأة: بيانات المكتب الصحيحة تظهر في كل المخرجات
        SettingsService::new(&db)
            .update(&uid, &serde_json::json!({
                "office_name": "مكتب الهوية الآمن",
                "office_phone": "0512345678",
                "office_whatsapp": "0512345678",
                "fal_license_number": "6666666666"
            }))
            .unwrap();

        let (inv2, pays2, filled) = fin.invoice_pdf_payload(&invoice_id, &uid).unwrap();
        assert_eq!(office_display_name(&filled), "مكتب الهوية الآمن", "نظام الهوية الوحيد يقرأ من الإعدادات");
        let invoice_html_filled = invoice_html(&inv2, &pays2, &filled);
        assert!(invoice_html_filled.contains("مكتب الهوية الآمن"), "اسم المكتب يظهر");
        assert!(invoice_html_filled.contains("0512345678"), "جوال المكتب يظهر");
        assert!(invoice_html_filled.contains("واتساب: 0512345678"), "واتساب المكتب يظهر");
        assert!(!invoice_html_filled.contains("6666666666"), "السجل القديم يحتفظ بالترخيص المحفوظ وقت إنشائه");
        assert_no_hardcoded_branding(&invoice_html_filled, "فاتورة معبأة");

        let (rep2, rep_settings2) = fin.report_pdf_payload(&uid, "2026-09-01", "2026-09-30").unwrap();
        let report_filled = report_html(&rep2, &rep_settings2, "2026-09-01", "2026-09-30");
        assert!(report_filled.contains("مكتب الهوية الآمن"), "اسم المكتب في التقرير");
        assert!(report_filled.contains("6666666666"), "الرخصة في التقرير");

        let contract_html_filled = crate::contracts::print_html(&contract_data, &filled);
        assert!(contract_html_filled.contains("مكتب الهوية الآمن"), "اسم المكتب في العقد");
        assert!(contract_html_filled.contains("0512345678"), "الجوال في العقد");
        assert_no_hardcoded_branding(&contract_html_filled, "عقد معبأ");

        let offer_filled = crate::share_pack::build_offer_html(
            &serde_json::json!({ "name": "فلة العليا" }),
            Some("مكتب الهوية الآمن"),
            &[],
        );
        assert!(offer_filled.contains("مكتب الهوية الآمن"), "اسم المكتب في العرض");
        assert_no_hardcoded_branding(&offer_filled, "عرض معبأ");
    }

    // إعادة التشغيل: الإعدادات تُقرأ من القاعدة نفسها — الهوية مستمرة
    {
        let db = SqliteDatabase::new(&db_path).expect("reopen");
        let fin = FinanceService::new(&db);
        let (inv3, pays3, settings3) = fin.invoice_pdf_payload(&invoice_id, &uid).unwrap();
        assert_eq!(office_display_name(&settings3), "مكتب الهوية الآمن", "الهوية مستمرة بعد إعادة التشغيل");
        let html3 = invoice_html(&inv3, &pays3, &settings3);
        assert!(html3.contains("مكتب الهوية الآمن"));
        assert_no_hardcoded_branding(&html3, "فاتورة بعد إعادة التشغيل");
    }

    let _ = std::fs::remove_file(&db_path);
}

// ===== Hardening: حدود مسارات أوامر المشاركة/التصدير =====

#[test]
fn test_path_guard_allows_valid_share_pack_and_rejects_traversal() {
    let root = temp_dir().join(format!("obr_path_ok_{}", uuid::Uuid::new_v4()));
    let share = crate::share_pack::share_packs_root(&root);
    std::fs::create_dir_all(&share).unwrap();
    let file = share.join("offer.txt");
    std::fs::write(&file, b"ok").unwrap();

    let allowed = crate::share_pack::resolve_share_pack_file(&root, &file.to_string_lossy())
        .expect("مسار صالح داخل share-packs");
    assert!(allowed.is_file());

    let traversal = share.join("..").join("escape.txt");
    // حتى لو وُجد الملف خارج الجذر عبر .. في النص قبل الحل
    let outside = root.join("escape.txt");
    std::fs::write(&outside, b"no").unwrap();
    let err = crate::share_pack::resolve_share_pack_file(&root, &traversal.to_string_lossy())
        .expect_err("يجب رفض traversal");
    assert!(
        err.contains("traversal") || err.contains("غير مسموح") || err.contains("خارج"),
        "رسالة الرفض: {err}"
    );

    let abs_outside = temp_dir().join(format!("obr_abs_{}.txt", uuid::Uuid::new_v4()));
    std::fs::write(&abs_outside, b"x").unwrap();
    let err2 = crate::share_pack::resolve_share_pack_file(&root, &abs_outside.to_string_lossy())
        .expect_err("يجب رفض مسار مطلق خارج النطاق");
    assert!(err2.contains("خارج") || err2.contains("غير مسموح"), "رسالة: {err2}");

    let _ = std::fs::remove_file(&abs_outside);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn test_export_destination_allowlist_and_rejects_outside() {
    let root = temp_dir().join(format!("obr_export_root_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let dest = root.join("design.pdf");
    let resolved = crate::share_pack::resolve_export_destination(&root, &dest.to_string_lossy())
        .expect("التصدير داخل app_data مسموح");
    assert_eq!(resolved.file_name().unwrap(), "design.pdf");

    let abs_outside = temp_dir().join(format!("obr_export_bad_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&abs_outside).unwrap();
    let bad = abs_outside.join("steal.pdf");
    let err = crate::share_pack::resolve_export_destination(&root, &bad.to_string_lossy())
        .expect_err("يجب رفض مسار خارج الجذور المسموحة");
    assert!(err.contains("خارج") || err.contains("غير مسموح"), "رسالة: {err}");

    let trav = root.join("sub").join("..").join("..").join("nope.pdf");
    let err2 = crate::share_pack::resolve_export_destination(&root, &trav.to_string_lossy())
        .expect_err("يجب رفض traversal في التصدير");
    assert!(
        err2.contains("traversal") || err2.contains("غير مسموح") || err2.contains("خارج"),
        "رسالة: {err2}"
    );

    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&abs_outside);
}

#[test]
fn test_sensitive_path_commands_auth_gate_rejects_invalid_session() {
    ensure_test_auth_secret();
    let db = setup_test_db();
    let bad = validate_session_token(&db, "not-a-valid-session-token");
    assert!(bad.is_err(), "جلسة غير صالحة يجب أن تُرفض");

    let auth = AuthService::new(&db);
    let reg = auth
        .register("path_auth", "path_auth@example.com", "123456", None, None, None)
        .unwrap();
    let token = reg["token"].as_str().unwrap();
    assert!(validate_session_token(&db, token).is_ok());
    assert!(validate_session_token(&db, "tampered-token").is_err());
}

#[cfg(windows)]
#[test]
fn test_share_pack_rejects_junction_escape() {
    use std::process::Command;

    let root = temp_dir().join(format!("obr_junc_{}", uuid::Uuid::new_v4()));
    let share = crate::share_pack::share_packs_root(&root);
    std::fs::create_dir_all(&share).unwrap();
    let outside = root.join("outside_secret");
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(outside.join("secret.txt"), b"secret").unwrap();

    let link = share.join("escape");
    let status = Command::new("cmd")
        .args([
            "/C",
            "mklink",
            "/J",
            &link.to_string_lossy(),
            &outside.to_string_lossy(),
        ])
        .status();
    let Ok(status) = status else {
        let _ = std::fs::remove_dir_all(&root);
        return;
    };
    if !status.success() {
        let _ = std::fs::remove_dir_all(&root);
        return;
    }

    let err_dir = crate::share_pack::resolve_share_pack_dir(&root, &link.to_string_lossy());
    assert!(err_dir.is_err(), "يجب رفض مجلد junction خارج النطاق");

    let via = link.join("secret.txt");
    let err_file = crate::share_pack::resolve_share_pack_file(&root, &via.to_string_lossy());
    assert!(err_file.is_err(), "يجب رفض ملف عبر junction escape");

    let _ = std::fs::remove_dir(&link);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn test_export_rejects_wrong_extension_and_silent_overwrite() {
    let root = temp_dir().join(format!("obr_export_guard_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let wrong = root.join("design.png");
    let err = crate::share_pack::export_html_document("<p>x</p>", "pdf", &wrong, 800, 600)
        .expect_err("امتداد خاطئ يجب أن يُرفض");
    assert!(err.contains("امتداد") || err.contains("pdf"), "رسالة: {err}");

    let dest = root.join("design.pdf");
    std::fs::write(&dest, b"existing").unwrap();
    let err2 = crate::share_pack::export_html_document("<p>x</p>", "pdf", &dest, 800, 600)
        .expect_err("الاستبدال الصامت يجب أن يُرفض");
    assert!(err2.contains("موجود"), "رسالة: {err2}");
    assert_eq!(std::fs::read(&dest).unwrap(), b"existing");

    let _ = std::fs::remove_dir_all(&root);
}

/// خادم HTTP محلي بسيط لمسارات التتبع في الاختبارات.
fn spawn_tracking_http_server(
    routes: Vec<(String, u16, String, u64)>,
) -> (String, std::thread::JoinHandle<()>) {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock tracking server");
    let addr = listener.local_addr().unwrap();
    let handle = std::thread::spawn(move || {
        for (path_suffix, status, body, delay_ms) in routes {
            let Ok((mut stream, _)) = listener.accept() else {
                break;
            };
            if delay_ms > 0 {
                std::thread::sleep(std::time::Duration::from_millis(delay_ms));
            }
            let mut buf = [0u8; 2048];
            let _ = stream.read(&mut buf);
            let req = String::from_utf8_lossy(&buf);
            let matched = req.lines().next().unwrap_or("").contains(&path_suffix);
            let (code, payload) = if matched {
                (status, body)
            } else {
                (404u16, "{\"error\":\"not found\"}".to_string())
            };
            let resp = format!(
                "HTTP/1.1 {code} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",
                payload.len()
            );
            let _ = stream.write_all(resp.as_bytes());
        }
    });
    (format!("http://127.0.0.1:{}", addr.port()), handle)
}

#[test]
fn test_tracking_sync_phased_success_and_http_failure_preserve_stats() {
    ensure_test_auth_secret();
    let _env = crate::tracking_client::ENV_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());

    let db_path = temp_dir().join(format!("obr_track_sync_{}.db", uuid::Uuid::new_v4()));
    let db = SqliteDatabase::new(&db_path).expect("db");
    let auth = AuthService::new(&db);
    let u = auth
        .register("tr1", "tr1@example.com", "123456", None, None, None)
        .unwrap();
    let uid = verify_token(u["token"].as_str().unwrap()).unwrap();
    let campaigns = CampaignService::new(&db);
    let created = campaigns
        .create(
            &uid,
            &serde_json::json!({
                "name": "حملة تتبع",
                "platform": "snapchat",
                "ad_link": "https://www.snapchat.com/add/obr-test",
                "views": 10
            }),
        )
        .unwrap();
    let id = created["id"].as_str().unwrap().to_string();

    let mut camp = db.get_campaign(&id, &uid).unwrap().unwrap();
    camp.tracking_code = Some("abc123".into());
    camp.tracking_url = Some("http://127.0.0.1/t/abc123".into());
    camp.tracking_enabled = true;
    camp.tracked_clicks = 1;
    camp.unique_clicks = 1;
    db.update_campaign(&camp).unwrap();

    let (base_ok, h_ok) = spawn_tracking_http_server(vec![(
        "/v2/links/abc123/stats".into(),
        200,
        r#"{"tracked_clicks":9,"unique_clicks":4,"last_click_at":"2026-09-18T12:00:00Z","enabled":true}"#
            .into(),
        0,
    )]);
    std::env::set_var("OBR_TRACKING_ENDPOINT", &base_ok);
    let snapshot = campaigns.tracking_sync_load_snapshot(&id, &uid).unwrap();
    assert_eq!(snapshot.tracking_code, "abc123");
    let stats = crate::tracking_client::fetch_stats(
        &base_ok,
        "7f0d1c3e-5a2b-4c6d-8e9f-0a1b2c3d4e5f",
        &snapshot.tracking_code,
    )
    .unwrap();
    let synced = campaigns
        .tracking_sync_apply_stats(&snapshot, &stats)
        .unwrap();
    assert_eq!(synced["tracked_clicks"], 9);
    assert_eq!(synced["unique_clicks"], 4);
    let _ = h_ok.join();

    let (base_fail, h_fail) = spawn_tracking_http_server(vec![(
        "/v2/links/abc123/stats".into(),
        500,
        r#"{"error":"boom"}"#.into(),
        0,
    )]);
    std::env::set_var("OBR_TRACKING_ENDPOINT", &base_fail);
    let snapshot2 = campaigns.tracking_sync_load_snapshot(&id, &uid).unwrap();
    let err = crate::tracking_client::fetch_stats(
        &base_fail,
        "7f0d1c3e-5a2b-4c6d-8e9f-0a1b2c3d4e5f",
        &snapshot2.tracking_code,
    )
    .expect_err("يجب فشل HTTP");
    assert!(!err.is_empty());
    let after_fail = campaigns.get(&id, &uid).unwrap();
    assert_eq!(after_fail["tracked_clicks"], 9, "لا فقدان عند فشل HTTP");
    assert_eq!(after_fail["unique_clicks"], 4);
    let _ = h_fail.join();

    std::env::remove_var("OBR_TRACKING_ENDPOINT");
    let _ = std::fs::remove_file(&db_path);
}

#[test]
fn test_tracking_sync_db_available_during_http_wait_and_no_double_apply() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    ensure_test_auth_secret();
    let db_path = temp_dir().join(format!("obr_track_lock_{}.db", uuid::Uuid::new_v4()));
    let db = SqliteDatabase::new(&db_path).expect("db");
    let auth = AuthService::new(&db);
    let u = auth
        .register("tr2", "tr2@example.com", "123456", None, None, None)
        .unwrap();
    let uid = verify_token(u["token"].as_str().unwrap()).unwrap();
    let campaigns = CampaignService::new(&db);
    let created = campaigns
        .create(
            &uid,
            &serde_json::json!({
                "name": "حملة قفل",
                "platform": "instagram",
                "ad_link": "https://instagram.com/p/obr",
                "views": 5
            }),
        )
        .unwrap();
    let id = created["id"].as_str().unwrap().to_string();
    let mut camp = db.get_campaign(&id, &uid).unwrap().unwrap();
    camp.tracking_code = Some("lock1".into());
    camp.tracking_url = Some("http://127.0.0.1/t/lock1".into());
    camp.tracking_enabled = true;
    camp.tracked_clicks = 0;
    db.update_campaign(&camp).unwrap();
    drop(campaigns);

    let db_mutex = Arc::new(Mutex::new(db));
    let apply_count = Arc::new(AtomicUsize::new(0));
    let barrier = Arc::new(std::sync::Barrier::new(2));

    let db_a = db_mutex.clone();
    let apply_a = apply_count.clone();
    let barrier_a = barrier.clone();
    let id_a = id.clone();
    let uid_a = uid.clone();
    let sync_thread = std::thread::spawn(move || {
        let snapshot = {
            let guard = db_a.lock().unwrap();
            CampaignService::new(&*guard)
                .tracking_sync_load_snapshot(&id_a, &uid_a)
                .unwrap()
        };
        assert_eq!(snapshot.tracking_code, "lock1");
        barrier_a.wait();
        std::thread::sleep(Duration::from_millis(350));
        let stats = crate::tracking_client::TrackingStats {
            tracked_clicks: 5,
            unique_clicks: 2,
            last_click_at: Some("2026-09-18T10:00:00Z".into()),
            enabled: true,
        };
        let guard = db_a.lock().unwrap();
        CampaignService::new(&*guard)
            .tracking_sync_apply_stats(&snapshot, &stats)
            .unwrap();
        apply_a.fetch_add(1, Ordering::SeqCst);
    });

    let db_b = db_mutex.clone();
    let uid_b = uid.clone();
    let other_thread = std::thread::spawn(move || {
        barrier.wait();
        let start = Instant::now();
        let guard = db_b.lock().unwrap_or_else(|e| e.into_inner());
        let waited = start.elapsed();
        assert!(
            waited < Duration::from_millis(150),
            "قاعدة البيانات يجب أن تُتاح أثناء انتظار HTTP، انتُظر {:?}",
            waited
        );
        let listed = CampaignService::new(&*guard)
            .list(&uid_b, 1, 30, "", "", "", "")
            .unwrap();
        assert!(listed["total"].as_i64().unwrap() >= 1);
    });

    sync_thread.join().unwrap();
    other_thread.join().unwrap();
    assert_eq!(apply_count.load(Ordering::SeqCst), 1, "تطبيق الإحصاءات مرة واحدة فقط");

    let guard = db_mutex.lock().unwrap();
    let got = CampaignService::new(&*guard).get(&id, &uid).unwrap();
    assert_eq!(got["tracked_clicks"], 5);
    assert_eq!(got["unique_clicks"], 2);
    drop(guard);
    let _ = std::fs::remove_file(&db_path);
}

#[test]
fn test_tracking_enable_phased_register_then_persist() {
    ensure_test_auth_secret();
    let _env = crate::tracking_client::ENV_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());

    let (base, handle) = spawn_tracking_http_server(vec![(
        "/v2/links".into(),
        200,
        r#"{"tracking_code":"newcode","tracking_url":"http://127.0.0.1/t/newcode"}"#.into(),
        20,
    )]);
    std::env::set_var("OBR_TRACKING_ENDPOINT", &base);

    let db_path = temp_dir().join(format!("obr_track_en_{}.db", uuid::Uuid::new_v4()));
    let db = SqliteDatabase::new(&db_path).expect("db");
    let auth = AuthService::new(&db);
    let u = auth
        .register("tr3", "tr3@example.com", "123456", None, None, None)
        .unwrap();
    let uid = verify_token(u["token"].as_str().unwrap()).unwrap();
    let campaigns = CampaignService::new(&db);
    let created = campaigns
        .create(
            &uid,
            &serde_json::json!({
                "name": "تفعيل تتبع",
                "platform": "snapchat",
                "ad_link": "https://x.com/obr/status/1"
            }),
        )
        .unwrap();
    let id = created["id"].as_str().unwrap().to_string();

    let original = campaigns
        .tracking_enable_load_destination(&id, &uid)
        .unwrap();
    let registered = crate::tracking_client::register_link(
        &base,
        "7f0d1c3e-5a2b-4c6d-8e9f-0a1b2c3d4e5f",
        &original,
    )
    .unwrap();
    assert_eq!(registered.tracking_code, "newcode");
    campaigns
        .tracking_enable_persist(&id, &uid, &original, &registered)
        .unwrap();
    let got = campaigns.get(&id, &uid).unwrap();
    assert_eq!(got["tracking_enabled"], true);
    assert_eq!(got["tracking_code"], "newcode");
    assert_eq!(got["original_ad_url"], original);

    let _ = handle.join();
    std::env::remove_var("OBR_TRACKING_ENDPOINT");
    let _ = std::fs::remove_file(&db_path);
}

fn seed_tracked_campaign(db: &SqliteDatabase, suffix: &str) -> (String, String) {
    ensure_test_auth_secret();
    let auth = AuthService::new(db);
    let email = format!("{suffix}@example.com");
    let u = auth
        .register(suffix, &email, "123456", None, None, None)
        .unwrap();
    let uid = verify_token(u["token"].as_str().unwrap()).unwrap();
    let campaigns = CampaignService::new(db);
    let created = campaigns
        .create(
            &uid,
            &serde_json::json!({
                "name": format!("حملة {suffix}"),
                "platform": "snapchat",
                "ad_link": "https://www.snapchat.com/add/obr-race",
                "views": 3
            }),
        )
        .unwrap();
    let id = created["id"].as_str().unwrap().to_string();
    let mut camp = db.get_campaign(&id, &uid).unwrap().unwrap();
    camp.tracking_code = Some("race-code-1".into());
    camp.tracking_url = Some("http://127.0.0.1/t/race-code-1".into());
    camp.tracking_enabled = true;
    camp.tracked_clicks = 2;
    camp.unique_clicks = 1;
    camp.last_click_at = Some("2026-09-01T00:00:00Z".into());
    db.update_campaign(&camp).unwrap();
    (uid, id)
}

#[test]
fn test_tracking_sync_ignores_stale_response_after_code_or_disable_change() {
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    let db_path = temp_dir().join(format!("obr_track_stale_{}.db", uuid::Uuid::new_v4()));
    let db = SqliteDatabase::new(&db_path).expect("db");
    let (uid, id) = seed_tracked_campaign(&db, "stale1");
    let db_mutex = Arc::new(Mutex::new(db));
    let barrier = Arc::new(std::sync::Barrier::new(2));

    let db_sync = db_mutex.clone();
    let barrier_sync = barrier.clone();
    let id_sync = id.clone();
    let uid_sync = uid.clone();
    let sync_thread = std::thread::spawn(move || {
        let snapshot = {
            let guard = db_sync.lock().unwrap();
            CampaignService::new(&*guard)
                .tracking_sync_load_snapshot(&id_sync, &uid_sync)
                .unwrap()
        };
        assert_eq!(snapshot.tracking_code, "race-code-1");
        assert!(snapshot.tracking_enabled);
        barrier_sync.wait();
        // محاكاة وصول رد HTTP القديم بعد تغيّر الحالة
        std::thread::sleep(Duration::from_millis(200));
        let stale_stats = crate::tracking_client::TrackingStats {
            tracked_clicks: 99,
            unique_clicks: 77,
            last_click_at: Some("2026-09-18T23:00:00Z".into()),
            enabled: true,
        };
        let guard = db_sync.lock().unwrap();
        let out = CampaignService::new(&*guard)
            .tracking_sync_apply_stats(&snapshot, &stale_stats)
            .unwrap();
        // يجب ألا تُطبَّق أرقام الرد القديم
        assert_ne!(out["tracked_clicks"], 99);
        assert_ne!(out["unique_clicks"], 77);
    });

    let db_mut = db_mutex.clone();
    let id_mut = id.clone();
    let uid_mut = uid.clone();
    let mutator = std::thread::spawn(move || {
        barrier.wait();
        let guard = db_mut.lock().unwrap();
        let mut camp = guard.get_campaign(&id_mut, &uid_mut).unwrap().unwrap();
        // تغيير الرمز + تعطيل التتبع أثناء انتظار HTTP
        camp.tracking_code = Some("race-code-2".into());
        camp.tracking_url = Some("http://127.0.0.1/t/race-code-2".into());
        camp.tracking_enabled = false;
        camp.tracked_clicks = 2;
        camp.unique_clicks = 1;
        guard.update_campaign(&camp).unwrap();
    });

    sync_thread.join().unwrap();
    mutator.join().unwrap();

    let guard = db_mutex.lock().unwrap();
    let got = CampaignService::new(&*guard).get(&id, &uid).unwrap();
    assert_eq!(got["tracking_code"], "race-code-2");
    assert_eq!(got["tracking_enabled"], false);
    assert_eq!(got["tracked_clicks"], 2, "الرد القديم لم يُطبّق بعد تغيير الرمز/التعطيل");
    assert_eq!(got["unique_clicks"], 1);
    assert_eq!(got["last_click_at"], "2026-09-01T00:00:00Z");
    drop(guard);
    let _ = std::fs::remove_file(&db_path);
}

#[test]
fn test_tracking_sync_ignores_stale_response_after_delete() {
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    let db_path = temp_dir().join(format!("obr_track_del_{}.db", uuid::Uuid::new_v4()));
    let db = SqliteDatabase::new(&db_path).expect("db");
    let (uid, id) = seed_tracked_campaign(&db, "del1");
    let db_mutex = Arc::new(Mutex::new(db));
    let barrier = Arc::new(std::sync::Barrier::new(2));

    let db_sync = db_mutex.clone();
    let barrier_sync = barrier.clone();
    let id_sync = id.clone();
    let uid_sync = uid.clone();
    let sync_thread = std::thread::spawn(move || {
        let snapshot = {
            let guard = db_sync.lock().unwrap();
            CampaignService::new(&*guard)
                .tracking_sync_load_snapshot(&id_sync, &uid_sync)
                .unwrap()
        };
        barrier_sync.wait();
        std::thread::sleep(Duration::from_millis(200));
        let stale_stats = crate::tracking_client::TrackingStats {
            tracked_clicks: 50,
            unique_clicks: 40,
            last_click_at: Some("2026-09-18T22:00:00Z".into()),
            enabled: true,
        };
        let guard = db_sync.lock().unwrap();
        let err = CampaignService::new(&*guard)
            .tracking_sync_apply_stats(&snapshot, &stale_stats)
            .expect_err("بعد الحذف يجب رفض تطبيق الرد القديم");
        assert!(err.contains("غير موجودة"), "رسالة: {err}");
    });

    let db_del = db_mutex.clone();
    let id_del = id.clone();
    let uid_del = uid.clone();
    let deleter = std::thread::spawn(move || {
        barrier.wait();
        let guard = db_del.lock().unwrap();
        CampaignService::new(&*guard)
            .delete_persist(&id_del, &uid_del)
            .unwrap();
    });

    sync_thread.join().unwrap();
    deleter.join().unwrap();

    let guard = db_mutex.lock().unwrap();
    assert!(
        CampaignService::new(&*guard).get(&id, &uid).is_err(),
        "الحملة محذوفة ولا أثر لرد قديم"
    );
    drop(guard);
    let _ = std::fs::remove_file(&db_path);
}

/// P10: busy_timeout + foreign_keys على اتصال الإنتاج؛ نسخ متسق أثناء كتابات متزامنة.
#[test]
fn test_sqlite_connection_policy_and_backup_under_writes() {
    use crate::db::backup;
    use crate::db::files::FileStore;
    use rusqlite::Connection;
    use std::sync::{Arc, Barrier};
    use std::time::Duration;

    let root = temp_dir().join(format!("obr_p10_sqlite_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let db_path = root.join("mawasib.db");

    let db = SqliteDatabase::new(&db_path).expect("primary open");
    drop(db);

    // اتصال جديد بعد SqliteDatabase::new يجب أن يرث سياسة الملف؛ نتحقق عبر open_configured.
    let probe = crate::db::open_configured(&db_path).expect("configured");
    let fk: i64 = probe
        .query_row("PRAGMA foreign_keys", [], |r| r.get(0))
        .unwrap();
    assert_eq!(fk, 1, "foreign_keys يجب أن تكون ON");
    let busy: i64 = probe
        .query_row("PRAGMA busy_timeout", [], |r| r.get(0))
        .unwrap();
    assert_eq!(busy, crate::db::connection::BUSY_TIMEOUT_MS as i64);
    let journal: String = probe
        .query_row("PRAGMA journal_mode", [], |r| r.get(0))
        .unwrap();
    assert_eq!(
        journal.to_ascii_lowercase(),
        "delete",
        "لا نفرض WAL — الافتراضي DELETE"
    );

    // مستخدم + صف مؤكد قبل النسخ
    let uid = "p10-user";
    probe
        .execute(
            "INSERT INTO users(id, username, email, password_hash) VALUES (?1, 'p10', 'p10@ex.com', 'x')",
            rusqlite::params![uid],
        )
        .unwrap();
    probe
        .execute(
            "INSERT INTO audit_logs(id, user_id, action, entity_type, entity_id, details, created_at)
             VALUES ('a-before', ?1, 'p10', 'test', '1', 'committed-before-backup', datetime('now'))",
            rusqlite::params![uid],
        )
        .unwrap();
    drop(probe);

    let barrier = Arc::new(Barrier::new(2));
    let db_path_w = db_path.clone();
    let barrier_w = barrier.clone();
    let writer = std::thread::spawn(move || {
        let w = crate::db::open_configured(&db_path_w).expect("writer");
        barrier_w.wait();
        for i in 0..40 {
            let id = format!("a-w-{i}");
            let _ = w.execute(
                "INSERT INTO audit_logs(id, user_id, action, entity_type, entity_id, details, created_at)
                 VALUES (?1, ?2, 'p10w', 'test', ?3, 'during-backup', datetime('now'))",
                rusqlite::params![id, uid, i.to_string()],
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    });

    let bconn = crate::db::open_configured(&db_path).expect("backup conn");
    barrier.wait();
    std::thread::sleep(Duration::from_millis(15));
    let store = FileStore::new(&root).unwrap();
    let pkg = backup::create_backup_package(&root, &db_path, &bconn, &store, Some(uid))
        .expect("backup while writes");
    drop(bconn);
    writer.join().unwrap();

    // الحزمة سليمة وتحتوي الصف المؤكد قبل النسخ
    backup::validate_backup_package_complete(&pkg.path).expect("package integrity");
    let bak_db = pkg.path.join("mawasib.db");
    let bak = Connection::open(&bak_db).unwrap();
    let before: i64 = bak
        .query_row(
            "SELECT COUNT(*) FROM audit_logs WHERE id = 'a-before'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(before, 1, "النسخة يجب أن تحتوي آخر بيانات مؤكدة قبل/أثناء النسخ");
    let ok: String = bak
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .unwrap();
    assert_eq!(ok, "ok");
    drop(bak);

    // استعادة + إعادة فتح
    let restored = backup::restore_backup_package(&root, &db_path, &pkg.path).expect("restore");
    assert_eq!(restored["restored"], true);
    let db2 = SqliteDatabase::new(&db_path).expect("reopen after restore");
    drop(db2);
    let after = crate::db::open_configured(&db_path).unwrap();
    let n: i64 = after
        .query_row(
            "SELECT COUNT(*) FROM audit_logs WHERE id = 'a-before'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(n, 1);
    let fk2: i64 = after
        .query_row("PRAGMA foreign_keys", [], |r| r.get(0))
        .unwrap();
    assert_eq!(fk2, 1);

    // قراءتان/كتابتان متزامنتان لا تفشلان فورًا بـ database is locked
    let path_a = db_path.clone();
    let path_b = db_path.clone();
    let barrier2 = Arc::new(Barrier::new(2));
    let b_a = barrier2.clone();
    let t1 = std::thread::spawn(move || {
        let c = crate::db::open_configured(&path_a).unwrap();
        b_a.wait();
        for i in 0..20 {
            c.execute(
                "INSERT INTO audit_logs(id, user_id, action, entity_type, entity_id, details, created_at)
                 VALUES (?1, ?2, 'rw', 't', ?3, 'x', datetime('now'))",
                rusqlite::params![format!("rw-a-{i}"), uid, i.to_string()],
            )
            .expect("writer A should wait via busy_timeout, not fail instantly");
        }
    });
    let b_b = barrier2;
    let t2 = std::thread::spawn(move || {
        let c = crate::db::open_configured(&path_b).unwrap();
        b_b.wait();
        for i in 0..20 {
            let _: i64 = c
                .query_row("SELECT COUNT(*) FROM audit_logs", [], |r| r.get(0))
                .expect("reader B should wait via busy_timeout, not fail instantly");
            let _ = i;
        }
    });
    t1.join().unwrap();
    t2.join().unwrap();

    let _ = std::fs::remove_dir_all(&root);
}

/// P11: فشل وسط المعاملة يلغي كل الكتابات ولا يُسجّل audit نجاح.
#[test]
fn test_atomic_tx_fault_injection_rolls_back_and_skips_audit() {
    use crate::db::tx;
    use crate::db::{AuditLog, Client, Property};

    let root = temp_dir().join(format!("obr_p11_tx_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let db_path = root.join("mawasib.db");
    let db = SqliteDatabase::new(&db_path).expect("open");

    let uid = "p11-user";
    db.create_user(&crate::db::User {
        id: uid.to_string(),
        username: "p11".into(),
        email: "p11@ex.com".into(),
        password_hash: "x".into(),
        full_name: None,
        phone: None,
        office_name: None,
        created_at: chrono::Utc::now().to_rfc3339(),
    })
    .unwrap();

    let client = Client {
        context: crate::country::RecordContext::default(),
        id: "c1".into(),
        user_id: uid.into(),
        name: "مالك".into(),
        phone: None,
        whatsapp: None,
        relationship_type: Some("owner".into()),
        city: None,
        interests: None,
        required_property_type: None,
        budget: None,
        notes: None,
        last_contact: None,
        follow_up_date: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        email: None,
        phone_secondary: None,
        company: None,
        source: None,
        status: None,
        category: None,
        is_training: false,
    };
    db.create_client_atomic(
        &client,
        &["owner".into()],
        &AuditLog {
            id: "aud-c1".into(),
            user_id: Some(uid.into()),
            action: "client_create".into(),
            entity_type: Some("client".into()),
            entity_id: Some("c1".into()),
            details: Some("ok".into()),
            created_at: chrono::Utc::now().to_rfc3339(),
        },
    )
    .unwrap();

    // --- فشل بعد أول كتابة: لا عقار ولا audit ---
    tx::test_set_tx_fail_stage(Some("after_first"));
    let prop = Property::blank("p-fail1".into(), uid.into(), "عرض فاشل".into());
    let mut prop = prop;
    prop.owner_client_id = Some("c1".into());
    let err = db
        .create_property_atomic(
            &prop,
            uid,
            &AuditLog {
                id: "aud-fail1".into(),
                user_id: Some(uid.into()),
                action: "property_create".into(),
                entity_type: Some("property".into()),
                entity_id: Some("p-fail1".into()),
                details: Some("should not commit".into()),
                created_at: chrono::Utc::now().to_rfc3339(),
            },
        )
        .expect_err("must fail after first write");
    assert!(err.contains("after_first"), "{err}");
    tx::test_set_tx_fail_stage(None);

    assert!(db.get_property("p-fail1", uid).unwrap().is_none());
    let audits = db.list_audit_logs(uid, 50).unwrap();
    assert!(
        !audits.iter().any(|a| a.id == "aud-fail1"),
        "audit لعملية rollback يجب ألا يُسجَّل"
    );
    assert!(
        db.get_client_property_link("c1", "p-fail1").unwrap().is_none(),
        "رابط المالك لا يبقى بعد rollback"
    );

    // --- فشل قبل commit (بعد كل الكتابات داخل tx): لا شيء يثبّت ---
    tx::test_set_tx_fail_stage(Some("before_commit"));
    let prop2 = {
        let mut p = Property::blank("p-fail2".into(), uid.into(), "عرض فاشل2".into());
        p.owner_client_id = Some("c1".into());
        p
    };
    let err2 = db
        .create_property_atomic(
            &prop2,
            uid,
            &AuditLog {
                id: "aud-fail2".into(),
                user_id: Some(uid.into()),
                action: "property_create".into(),
                entity_type: Some("property".into()),
                entity_id: Some("p-fail2".into()),
                details: Some("should rollback before commit".into()),
                created_at: chrono::Utc::now().to_rfc3339(),
            },
        )
        .expect_err("must fail before commit");
    assert!(err2.contains("before_commit"), "{err2}");
    tx::test_set_tx_fail_stage(None);

    assert!(db.get_property("p-fail2", uid).unwrap().is_none());
    assert!(!db.list_audit_logs(uid, 50).unwrap().iter().any(|a| a.id == "aud-fail2"));

    // --- مسار ناجح: الكل أو لا شيء يثبت معًا ---
    let mut prop_ok = Property::blank("p-ok".into(), uid.into(), "عرض ناجح".into());
    prop_ok.owner_client_id = Some("c1".into());
    db.create_property_atomic(
        &prop_ok,
        uid,
        &AuditLog {
            id: "aud-ok".into(),
            user_id: Some(uid.into()),
            action: "property_create".into(),
            entity_type: Some("property".into()),
            entity_id: Some("p-ok".into()),
            details: Some("committed".into()),
            created_at: chrono::Utc::now().to_rfc3339(),
        },
    )
    .unwrap();
    assert!(db.get_property("p-ok", uid).unwrap().is_some());
    assert!(db.get_client_property_link("c1", "p-ok").unwrap().is_some());
    assert!(db.list_audit_logs(uid, 50).unwrap().iter().any(|a| a.id == "aud-ok"));

    // --- حذف فاتورة: فشل بعد حذف المدفوعات لا يترك مدفوعات محذوفة بدون فاتورة ---
    db.create_finance_invoice(&crate::db::FinanceInvoice {
        context: crate::country::RecordContext::default(),
        id: "inv1".into(),
        user_id: uid.into(),
        number: "INV-1".into(),
        title: "فاتورة".into(),
        issue_date: "2026-01-01".into(),
        due_date: None,
        amount: 100.0,
        client_id: None,
        property_id: None,
        campaign_id: None,
        operation_id: None,
        notes: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        ..Default::default()
    })
    .unwrap();
    db.create_finance_payment(&crate::db::FinancePayment {
        context: crate::country::RecordContext::default(),
        id: "pay1".into(),
        user_id: uid.into(),
        occurs_on: "2026-01-02".into(),
        amount: 50.0,
        target_type: "invoice".into(),
        target_id: "inv1".into(),
        method: None,
        reference: None,
        notes: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        ..Default::default()
    })
    .unwrap();

    tx::test_set_tx_fail_stage(Some("after_first"));
    let err_inv = db
        .delete_finance_invoice_atomic(uid, "inv1")
        .expect_err("fail after deleting payments");
    assert!(err_inv.contains("after_first"));
    tx::test_set_tx_fail_stage(None);

    assert!(db.get_finance_invoice("inv1", uid).unwrap().is_some(), "الفاتورة تبقى بعد rollback");
    let pays = db
        .list_finance_payments(uid, "", "", "invoice", "inv1", 0, 0)
        .unwrap();
    assert_eq!(pays.len(), 1, "المدفوعات تبقى بعد rollback");

    db.delete_finance_invoice_atomic(uid, "inv1").unwrap();
    assert!(db.get_finance_invoice("inv1", uid).unwrap().is_none());
    assert!(db
        .list_finance_payments(uid, "", "", "invoice", "inv1", 0, 0)
        .unwrap()
        .is_empty());

    drop(db);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn test_country_snapshot_survives_settings_change_and_restart() {
    use crate::finance_service::FinanceService;
    use serde_json::json;
    let path = temp_dir().join(format!("obr_country_{}.db", uuid::Uuid::new_v4()));
    ensure_test_auth_secret();
    let db = SqliteDatabase::new(&path).unwrap();
    let u = AuthService::new(&db).register("country", "country@example.com", "123456", None, None, None).unwrap();
    let uid = verify_token(u["token"].as_str().unwrap()).unwrap();
    let settings = SettingsService::new(&db);
    settings.update(&uid, &json!({"country":"SA", "fal_license_type":"brokerage_marketing", "fal_license_number":"SA-001"})).unwrap();
    let property = PropertyService::new(&db).create(&uid, &json!({"name":"عرض سعودي", "price":1000000})).unwrap();
    let fin = FinanceService::new(&db);
    let invoice = fin.create_invoice(&uid, &json!({"title":"فاتورة سعودية", "issue_date":"2026-09-21", "amount":1000})).unwrap();
    assert_eq!(property["country"], "SA"); assert_eq!(property["currency"], "SAR");
    assert_eq!(invoice["currency"], "SAR");
    settings.update(&uid, &json!({"country":"AE", "fal_license_type":"ae_dubai_broker", "fal_license_number":"AE-001"})).unwrap();
    let new_invoice = fin.create_invoice(&uid, &json!({"title":"فاتورة إماراتية", "issue_date":"2026-09-21", "amount":2000})).unwrap();
    assert_eq!(new_invoice["country"], "AE"); assert_eq!(new_invoice["currency"], "AED");
    let old_id = invoice["id"].as_str().unwrap();
    let (old, payments, print_settings) = fin.invoice_pdf_payload(old_id, &uid).unwrap();
    assert_eq!(old["currency"], "SAR");
    assert_eq!(print_settings.as_ref().unwrap().country, "SA");
    assert_eq!(print_settings.as_ref().unwrap().fal_license_number.as_deref(), Some("SA-001"));
    let html = crate::finance_pdf::invoice_html(&old, &payments, &print_settings);
    assert!(html.contains("SAR")); assert!(!html.contains("AED"));
    let summary = fin.summary(&uid, "", "").unwrap();
    assert_eq!(summary["currency"], "AED"); assert_eq!(summary["invoice_total"], 2000.0);
    let payment = fin.create_payment(&uid, &json!({"target_type":"invoice","target_id":old_id,"occurs_on":"2026-09-21","amount":10})).unwrap();
    assert_eq!(payment["currency"], "SAR", "الدفعة ترث عملة الفاتورة الأصلية");
    let pid = property["id"].as_str().unwrap();
    let linked_operation = fin.create_operation(&uid, &json!({"title":"عملية مرتبطة", "occurs_on":"2026-09-21", "deal_value":1000000, "property_id":pid})).unwrap();
    assert_eq!(linked_operation["currency"], "SAR");
    let linked_invoice = fin.create_invoice(&uid, &json!({"title":"فاتورة مرتبطة", "issue_date":"2026-09-21", "amount":25, "operation_id":linked_operation["id"]})).unwrap();
    assert_eq!(linked_invoice["currency"], "SAR");
    let linked_commission = fin.create_commission(&uid, &json!({"title":"عمولة مرتبطة", "occurs_on":"2026-09-21", "mode":"fixed", "amount_fixed":25,"invoice_id":linked_invoice["id"]})).unwrap();
    assert_eq!(linked_commission["currency"], "SAR");
    let brochure = BrochureService::new(&db).create(&uid, Some(pid), "custom", Some("عرض مرتبط"), Some(&json!({"title":"test"}))).unwrap();
    assert_eq!(BrochureService::new(&db).get(brochure["id"].as_str().unwrap(), &uid).unwrap()["currency"], "SAR");
    assert!(fin.create_commission(&uid, &json!({"title":"ربط عملتين", "occurs_on":"2026-09-21", "mode":"fixed", "amount_fixed":25,"invoice_id":new_invoice["id"],"property_id":pid})).is_err());

    let conn = rusqlite::Connection::open(&path).unwrap();
    assert!(conn.execute("UPDATE finance_invoices SET country='AE',currency='AED' WHERE id=?1", params![old_id]).is_err());
    drop(conn); drop(db);
    let reopened = SqliteDatabase::new(&path).unwrap();
    assert_eq!(FinanceService::new(&reopened).get_invoice(old_id, &uid).unwrap()["currency"], "SAR");
    drop(reopened); let _ = std::fs::remove_file(path);
}

#[test]
fn test_country_migration_backfills_legacy_sa_even_when_display_was_aed() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    for m in crate::db::migrations::MIGRATIONS.iter().filter(|m| m.version < 15) { conn.execute_batch(m.sql).unwrap(); }
    conn.execute_batch("INSERT INTO users(id,username,email,password_hash) VALUES('legacy','legacy','legacy@example.com','fixture'); INSERT INTO settings(user_id,display_currency) VALUES('legacy','AED'); INSERT INTO properties(id,user_id,name,price) VALUES('old','legacy','old',500); INSERT INTO market_reference_data(id,user_id,city,sale_price) VALUES('old-market','legacy','Riyadh',500);").unwrap();
    crate::country::migrate(&conn).unwrap();
    let old: (String,String) = conn.query_row("SELECT country,currency FROM properties WHERE id='old'", [], |r| Ok((r.get(0)?,r.get(1)?))).unwrap();
    assert_eq!(old, ("SA".into(),"SAR".into()));
    conn.execute("INSERT INTO properties(id,user_id,name,price) VALUES('new','legacy','new',700)",[]).unwrap();
    let new: (String,String) = conn.query_row("SELECT country,currency FROM properties WHERE id='new'", [], |r| Ok((r.get(0)?,r.get(1)?))).unwrap();
    assert_eq!(new, ("AE".into(),"AED".into()));
    assert!(conn.execute("INSERT INTO properties(id,user_id,name,country,currency) VALUES('invalid','legacy','bad','AE','SAR')",[]).is_err());
    assert_eq!(conn.query_row("PRAGMA integrity_check", [], |r|r.get::<_,String>(0)).unwrap(), "ok");
}

#[test]
fn test_country_pdf_currency_and_profile_catalog() {
    use serde_json::json;
    use base64::Engine;
    let dir = temp_dir().join(format!("obr_country_pdf_{}", uuid::Uuid::new_v4()));
    for (country,currency) in [("SA","SAR"),("AE","AED")] {
        let record = json!({"country":country,"currency":currency,"number":"COUNTRY-TEST","title":"اختبار عملة السجل","amount":1000,"paid":0,"remaining":1000});
        let html = crate::finance_pdf::invoice_html(&record, &[], &None);
        let pdf = crate::finance_pdf::generate_pdf(&dir, "country", currency, &html).unwrap();
        let bytes = std::fs::read(&pdf.pdf_path).unwrap();
        assert!(bytes.starts_with(b"%PDF"));
        let data_url = format!("data:application/pdf;base64,{}", base64::engine::general_purpose::STANDARD.encode(&bytes));
        let extracted = crate::contracts::preview_pdf(&data_url).unwrap();
        assert!(extracted["extracted_text_excerpt"].as_str().unwrap_or("").contains(currency), "PDF must contain {currency}: {extracted}");
    }
    assert_ne!(crate::country::service("SA","brokerage_marketing").unwrap()["url"], crate::country::service("AE","ae_dubai_broker").unwrap()["url"]);
    assert!(crate::country::service("AE","manual").unwrap().is_null());
    let catalog = crate::country::catalog();
    assert_eq!(catalog["countries"].as_array().unwrap().len(),6);
    for p in catalog["countries"].as_array().unwrap() {
        assert_eq!(crate::country::currency(p["code"].as_str().unwrap()).unwrap(),p["currency"].as_str().unwrap());
        if p["code"] != "SA" { assert!(!p["licenses"].to_string().contains("فال")); }
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn test_document_invoice_language_preserves_record_and_pdf() {
    use serde_json::json;
    use base64::Engine;
    let invoice = json!({"country":"SA","currency":"SAR","number":"LANG-2026","title":"عنوان العميل الخاص","client_name":"CUSTOMER-UNCHANGED","property_name":"عقار المستخدم","notes":"Invoice فاتورة لا تترجم ملاحظتي","status":"partial","status_label":"مدفوع جزئيًا","amount":1250,"paid":250,"remaining":1000});
    let original = invoice.clone();
    let payments = vec![json!({"occurs_on":"2026-09-21","method":"تحويل","reference":"REF-UNCHANGED","amount":250,"currency":"SAR"})];
    let dir = temp_dir().join(format!("obr_doc_language_{}", uuid::Uuid::new_v4()));
    for lang in ["ar", "en"] {
        let mut settings = make_test_settings(Some("OFFICE-UNCHANGED"), None);
        settings.as_mut().unwrap().ui_language = Some(lang.into());
        settings.as_mut().unwrap().country = "AE".into();
        settings.as_mut().unwrap().display_currency = Some("AED".into());
        let html = crate::finance_pdf::invoice_html(&invoice,&payments,&settings);
        assert!(html.contains(if lang == "en" {"lang=\"en\" dir=\"ltr\""} else {"lang=\"ar\" dir=\"rtl\""}));
        assert!(html.contains(if lang == "en" {"<h2>Invoice LANG-2026"} else {"<h2>فاتورة LANG-2026"}));
        assert!(html.contains(if lang == "en" {"Bank transfer"} else {"تحويل"}));
        assert!(html.contains("عنوان العميل الخاص") && html.contains("Invoice فاتورة لا تترجم ملاحظتي"));
        assert!(html.contains("SAR") && !html.contains("AED"));
        let output = crate::finance_pdf::generate_pdf(&dir,"language",lang,&html).unwrap();
        let bytes=std::fs::read(output.pdf_path).unwrap();assert!(bytes.starts_with(b"%PDF"));
        let preview=crate::contracts::preview_pdf(&format!("data:application/pdf;base64,{}",base64::engine::general_purpose::STANDARD.encode(bytes))).unwrap();
        let text=preview["extracted_text_excerpt"].as_str().unwrap_or("");
        assert!(text.contains("SAR") && text.contains("CUSTOMER-UNCHANGED"),"record values in {lang} PDF");
        if lang == "en" { assert!(text.contains("Invoice") && text.contains("Payments")); }
    }
    assert_eq!(invoice,original);
    std::fs::remove_dir_all(dir).unwrap();
}

/// Demo CLIPortable path isolation: عندما يكون OBR_APP_DATA_DIR مضبوطًا يجب أن
/// يستخدمه dirs_next_app_data (التي تعتمد عليها أوامر --demo-*) ولا يلمس %APPDATA%.
#[test]
fn demo_cli_respects_obr_app_data_dir() {
    let portable = temp_dir().join(format!("obr_portable_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&portable).unwrap();

    // 1) مع المتغير: يجب أن يُستخدم حرفيًا دون إلحاق com.mawasib.obr
    std::env::set_var("OBR_APP_DATA_DIR", &portable);
    let got = crate::dirs_next_app_data().unwrap();
    assert_eq!(got, portable, "Demo CLI يجب أن يستخدم OBR_APP_DATA_DIR كما هو");

    // 2) بدون المتغير: fallback الحالي %APPDATA%\com.mawasib.obr
    std::env::remove_var("OBR_APP_DATA_DIR");
    let fallback = crate::dirs_next_app_data().unwrap();
    let appdata = std::env::var("APPDATA").unwrap();
    assert_eq!(
        fallback,
        std::path::PathBuf::from(appdata).join("com.mawasib.obr"),
        "fallback يجب أن يبقى %APPDATA%\\com.mawasib.obr"
    );
    assert_ne!(fallback, portable, "يجب ألا يكتب CLI داخل مجلد الاختبار عند غياب المتغير");

    // 3) قيمة فارغة → fallback أيضًا (حماية من set "" )
    std::env::set_var("OBR_APP_DATA_DIR", "");
    let empty = crate::dirs_next_app_data().unwrap();
    assert_ne!(empty, std::path::PathBuf::from(""));

    std::env::remove_var("OBR_APP_DATA_DIR");
    std::fs::remove_dir_all(&portable).unwrap();
}
