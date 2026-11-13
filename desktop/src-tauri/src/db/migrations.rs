//! نظام Migrations إصداري لـ OBR
//! لا يُعتمد CREATE IF NOT EXISTS كبديل دائم بعد خط الأساس.

use rusqlite::Connection;

pub struct Migration {
    pub version: i32,
    pub name: &'static str,
    pub sql: &'static str,
}

/// V1 = خط الأساس المطابق للمخطط الحالي المستخدم في الإنتاج.
pub const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "baseline_core_tables",
        sql: r#"
            CREATE TABLE users (
                id TEXT PRIMARY KEY,
                username TEXT UNIQUE NOT NULL,
                email TEXT UNIQUE NOT NULL,
                password_hash TEXT NOT NULL,
                full_name TEXT,
                phone TEXT,
                office_name TEXT,
                created_at TEXT DEFAULT (datetime('now'))
            );

            CREATE TABLE licenses (
                id TEXT PRIMARY KEY,
                user_id TEXT UNIQUE NOT NULL,
                license_key TEXT UNIQUE NOT NULL,
                status TEXT DEFAULT 'active',
                issued_at TEXT DEFAULT (datetime('now')),
                expires_at TEXT,
                FOREIGN KEY (user_id) REFERENCES users(id)
            );

            CREATE TABLE sessions (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                token TEXT UNIQUE NOT NULL,
                device_name TEXT,
                created_at TEXT DEFAULT (datetime('now')),
                expires_at TEXT,
                FOREIGN KEY (user_id) REFERENCES users(id)
            );

            CREATE TABLE properties (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                name TEXT NOT NULL,
                city TEXT,
                district TEXT,
                location TEXT,
                property_type TEXT,
                area REAL,
                price REAL,
                price_per_meter REAL,
                ad_number TEXT,
                ad_link TEXT,
                owner_name TEXT,
                owner_phone TEXT,
                owner_contact TEXT,
                owner_notes TEXT,
                marketing_platform TEXT,
                marketing_link TEXT,
                published_date TEXT,
                last_updated TEXT,
                main_image TEXT,
                status TEXT DEFAULT 'active',
                created_at TEXT DEFAULT (datetime('now')),
                updated_at TEXT DEFAULT (datetime('now')),
                FOREIGN KEY (user_id) REFERENCES users(id)
            );

            CREATE TABLE property_images (
                id TEXT PRIMARY KEY,
                property_id TEXT NOT NULL,
                image_url TEXT NOT NULL,
                created_at TEXT DEFAULT (datetime('now')),
                FOREIGN KEY (property_id) REFERENCES properties(id) ON DELETE CASCADE
            );

            CREATE TABLE property_updates (
                id TEXT PRIMARY KEY,
                property_id TEXT NOT NULL,
                user_id TEXT NOT NULL,
                title TEXT,
                description TEXT,
                created_at TEXT DEFAULT (datetime('now')),
                FOREIGN KEY (property_id) REFERENCES properties(id) ON DELETE CASCADE
            );

            CREATE TABLE clients (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                name TEXT NOT NULL,
                phone TEXT,
                whatsapp TEXT,
                relationship_type TEXT,
                city TEXT,
                interests TEXT,
                required_property_type TEXT,
                budget TEXT,
                notes TEXT,
                last_contact TEXT,
                follow_up_date TEXT,
                created_at TEXT DEFAULT (datetime('now')),
                updated_at TEXT DEFAULT (datetime('now')),
                FOREIGN KEY (user_id) REFERENCES users(id)
            );

            CREATE TABLE communications (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                client_id TEXT NOT NULL,
                type TEXT,
                subject TEXT,
                notes TEXT,
                created_at TEXT DEFAULT (datetime('now')),
                FOREIGN KEY (user_id) REFERENCES users(id),
                FOREIGN KEY (client_id) REFERENCES clients(id) ON DELETE CASCADE
            );

            CREATE TABLE client_property_links (
                id TEXT PRIMARY KEY,
                client_id TEXT NOT NULL,
                property_id TEXT NOT NULL,
                link_type TEXT DEFAULT 'shared',
                created_at TEXT DEFAULT (datetime('now')),
                FOREIGN KEY (client_id) REFERENCES clients(id) ON DELETE CASCADE,
                FOREIGN KEY (property_id) REFERENCES properties(id) ON DELETE CASCADE,
                UNIQUE(client_id, property_id)
            );

            CREATE TABLE market_reference_data (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                city TEXT,
                district TEXT,
                property_type TEXT,
                usage_type TEXT,
                area REAL,
                sale_price REAL,
                price_per_meter_sale REAL,
                rent_value REAL,
                rent_per_meter REAL,
                requests_count INTEGER DEFAULT 0,
                offers_count INTEGER DEFAULT 0,
                entry_date TEXT,
                data_source TEXT,
                notes TEXT,
                created_at TEXT DEFAULT (datetime('now')),
                FOREIGN KEY (user_id) REFERENCES users(id)
            );

            CREATE TABLE brochures (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                property_id TEXT NOT NULL,
                template_id TEXT,
                title TEXT,
                content TEXT,
                created_at TEXT DEFAULT (datetime('now')),
                FOREIGN KEY (user_id) REFERENCES users(id),
                FOREIGN KEY (property_id) REFERENCES properties(id) ON DELETE CASCADE
            );

            CREATE TABLE settings (
                user_id TEXT PRIMARY KEY,
                office_name TEXT,
                office_logo TEXT,
                office_phone TEXT,
                office_whatsapp TEXT,
                office_email TEXT,
                office_address TEXT,
                broker_name TEXT,
                broker_phone TEXT,
                broker_whatsapp TEXT,
                default_brochure_template TEXT DEFAULT 'template1',
                fal_license_number TEXT,
                fal_license_type TEXT,
                updated_at TEXT DEFAULT (datetime('now')),
                FOREIGN KEY (user_id) REFERENCES users(id)
            );

            CREATE TABLE audit_logs (
                id TEXT PRIMARY KEY,
                user_id TEXT,
                action TEXT NOT NULL,
                entity_type TEXT,
                entity_id TEXT,
                details TEXT,
                created_at TEXT DEFAULT (datetime('now')),
                FOREIGN KEY (user_id) REFERENCES users(id)
            );
        "#,
    },
    Migration {
        version: 2,
        name: "core_search_indexes",
        sql: r#"
            CREATE INDEX idx_properties_user ON properties(user_id);
            CREATE INDEX idx_properties_status ON properties(status);
            CREATE INDEX idx_properties_city_district ON properties(city, district);
            CREATE INDEX idx_properties_name ON properties(name);
            CREATE INDEX idx_clients_user ON clients(user_id);
            CREATE INDEX idx_clients_phone ON clients(phone);
            CREATE INDEX idx_clients_name ON clients(name);
            CREATE INDEX idx_clients_follow_up ON clients(follow_up_date);
            CREATE INDEX idx_communications_client ON communications(client_id);
            CREATE INDEX idx_market_user_city ON market_reference_data(user_id, city, district);
            CREATE INDEX idx_brochures_user ON brochures(user_id);
            CREATE INDEX idx_audit_created ON audit_logs(created_at);
            CREATE INDEX idx_sessions_token ON sessions(token);
        "#,
    },
    Migration {
        version: 3,
        name: "files_and_backup_foundation",
        sql: r#"
            CREATE TABLE app_meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT DEFAULT (datetime('now'))
            );

            CREATE TABLE file_assets (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                entity_type TEXT,
                entity_id TEXT,
                original_name TEXT,
                stored_name TEXT NOT NULL,
                relative_path TEXT NOT NULL,
                mime_type TEXT,
                size_bytes INTEGER,
                checksum TEXT,
                missing INTEGER DEFAULT 0,
                created_at TEXT DEFAULT (datetime('now')),
                FOREIGN KEY (user_id) REFERENCES users(id)
            );

            CREATE INDEX idx_file_assets_entity ON file_assets(entity_type, entity_id);
            CREATE INDEX idx_file_assets_user ON file_assets(user_id);

            CREATE TABLE backup_history (
                id TEXT PRIMARY KEY,
                user_id TEXT,
                package_path TEXT NOT NULL,
                schema_version INTEGER NOT NULL,
                created_at TEXT DEFAULT (datetime('now')),
                notes TEXT,
                FOREIGN KEY (user_id) REFERENCES users(id)
            );

            INSERT INTO app_meta(key, value, updated_at) VALUES
              ('files_root_rel', 'files', datetime('now')),
              ('app_id', 'com.mawasib.obr', datetime('now'));
        "#,
    },
    Migration {
        version: 4,
        name: "demo_seed_batch_identification",
        sql: r#"
            CREATE TABLE seed_batches (
                id TEXT PRIMARY KEY,
                label TEXT NOT NULL,
                created_at TEXT DEFAULT (datetime('now')),
                notes TEXT
            );

            ALTER TABLE properties ADD COLUMN seed_batch_id TEXT;
            ALTER TABLE property_images ADD COLUMN seed_batch_id TEXT;
            ALTER TABLE property_updates ADD COLUMN seed_batch_id TEXT;
            ALTER TABLE clients ADD COLUMN seed_batch_id TEXT;
            ALTER TABLE communications ADD COLUMN seed_batch_id TEXT;
            ALTER TABLE client_property_links ADD COLUMN seed_batch_id TEXT;
            ALTER TABLE market_reference_data ADD COLUMN seed_batch_id TEXT;
            ALTER TABLE brochures ADD COLUMN seed_batch_id TEXT;
            ALTER TABLE audit_logs ADD COLUMN seed_batch_id TEXT;

            CREATE INDEX idx_properties_seed_batch ON properties(seed_batch_id);
            CREATE INDEX idx_clients_seed_batch ON clients(seed_batch_id);
            CREATE INDEX idx_market_seed_batch ON market_reference_data(seed_batch_id);
            CREATE INDEX idx_brochures_seed_batch ON brochures(seed_batch_id);
            CREATE INDEX idx_comms_seed_batch ON communications(seed_batch_id);
            CREATE INDEX idx_links_seed_batch ON client_property_links(seed_batch_id);
            CREATE INDEX idx_audit_seed_batch ON audit_logs(seed_batch_id);
        "#,
    },
    Migration {
        version: 5,
        name: "p2_1_contact_roles_and_client_fields",
        sql: r#"
            -- (1) إصلاح بيانات: الأعمدة التي يعتمدها التطبيق كنص إلزامي
            --     يجب ألا تحتوي NULL في القواعد القديمة، وإلا فشلت القراءة.
            UPDATE properties SET status = 'available' WHERE status IS NULL OR TRIM(status) = '';
            UPDATE properties SET created_at = COALESCE(created_at, datetime('now'));
            UPDATE properties SET updated_at = COALESCE(updated_at, created_at);
            UPDATE clients SET created_at = COALESCE(created_at, datetime('now'));
            UPDATE clients SET updated_at = COALESCE(updated_at, created_at);
            UPDATE clients SET name = '(بدون اسم)' WHERE name IS NULL OR TRIM(name) = '';
            UPDATE settings SET updated_at = COALESCE(updated_at, datetime('now'));
            UPDATE licenses SET status = 'active' WHERE status IS NULL OR TRIM(status) = '';
            UPDATE users SET created_at = COALESCE(created_at, datetime('now'));
            UPDATE sessions SET created_at = COALESCE(created_at, datetime('now'));
            UPDATE communications SET created_at = COALESCE(created_at, datetime('now'));
            UPDATE client_property_links SET created_at = COALESCE(created_at, datetime('now'));
            UPDATE property_images SET created_at = COALESCE(created_at, datetime('now'));
            UPDATE property_updates SET created_at = COALESCE(created_at, datetime('now'));
            UPDATE market_reference_data SET created_at = COALESCE(created_at, datetime('now'));
            UPDATE brochures SET created_at = COALESCE(created_at, datetime('now'));
            UPDATE audit_logs SET created_at = COALESCE(created_at, datetime('now'));

            -- (2) حقول جهة الاتصال الموسعة
            ALTER TABLE clients ADD COLUMN email TEXT;
            ALTER TABLE clients ADD COLUMN phone_secondary TEXT;
            ALTER TABLE clients ADD COLUMN company TEXT;
            ALTER TABLE clients ADD COLUMN source TEXT;
            ALTER TABLE clients ADD COLUMN status TEXT;
            ALTER TABLE clients ADD COLUMN category TEXT;

            -- (3) الأدوار المتعددة: علاقة حقيقية بدل حقل نصي واحد
            CREATE TABLE client_roles (
                id TEXT PRIMARY KEY,
                client_id TEXT NOT NULL,
                role TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                seed_batch_id TEXT,
                FOREIGN KEY (client_id) REFERENCES clients(id) ON DELETE CASCADE,
                UNIQUE(client_id, role)
            );
            CREATE INDEX idx_client_roles_client ON client_roles(client_id);
            CREATE INDEX idx_client_roles_role ON client_roles(role);
            CREATE INDEX idx_client_roles_seed_batch ON client_roles(seed_batch_id);
            CREATE INDEX idx_clients_status ON clients(status);
            CREATE INDEX idx_clients_email ON clients(email);

            -- (4) ترحيل الأدوار القائمة من relationship_type دون فقدان أي سجل.
            --     القيم القديمة غير المعروفة تُنقل كما هي ولا تُحذف.
            INSERT OR IGNORE INTO client_roles (id, client_id, role, created_at, seed_batch_id)
            SELECT
                lower(hex(randomblob(8))) || '-' || c.id,
                c.id,
                CASE LOWER(TRIM(c.relationship_type))
                    WHEN 'buyer'     THEN 'buyer'
                    WHEN 'مشتري'     THEN 'buyer'
                    WHEN 'tenant'    THEN 'tenant'
                    WHEN 'مستأجر'    THEN 'tenant'
                    WHEN 'owner'     THEN 'owner'
                    WHEN 'مالك'      THEN 'owner'
                    WHEN 'seller'    THEN 'owner'
                    WHEN 'investor'  THEN 'investor'
                    WHEN 'مستثمر'    THEN 'investor'
                    WHEN 'developer' THEN 'developer'
                    WHEN 'مطور'      THEN 'developer'
                    WHEN 'broker'    THEN 'broker'
                    WHEN 'وسيط'      THEN 'broker'
                    WHEN 'marketer'  THEN 'marketer'
                    WHEN 'مسوق'      THEN 'marketer'
                    WHEN 'office'    THEN 'company'
                    WHEN 'مكتب'      THEN 'company'
                    WHEN 'company'   THEN 'company'
                    WHEN 'شركة'      THEN 'company'
                    ELSE TRIM(c.relationship_type)
                END,
                COALESCE(c.created_at, datetime('now')),
                c.seed_batch_id
            FROM clients c
            WHERE c.relationship_type IS NOT NULL AND TRIM(c.relationship_type) <> '';
        "#,
    },
    Migration {
        version: 6,
        name: "p2_2_property_source_and_details",
        sql: r#"
            ALTER TABLE properties ADD COLUMN owner_client_id TEXT REFERENCES clients(id) ON DELETE SET NULL;
            ALTER TABLE properties ADD COLUMN source_type TEXT;
            ALTER TABLE properties ADD COLUMN purpose TEXT;
            ALTER TABLE properties ADD COLUMN usage_type TEXT;
            ALTER TABLE properties ADD COLUMN frontage TEXT;
            ALTER TABLE properties ADD COLUMN street_width REAL;
            ALTER TABLE properties ADD COLUMN streets_count INTEGER;
            ALTER TABLE properties ADD COLUMN latitude REAL;
            ALTER TABLE properties ADD COLUMN longitude REAL;
            ALTER TABLE properties ADD COLUMN description TEXT;
            ALTER TABLE properties ADD COLUMN listing_expiry_date TEXT;
            ALTER TABLE properties ADD COLUMN commission_agreement TEXT;
            ALTER TABLE properties ADD COLUMN fal_ad_license TEXT;
            ALTER TABLE properties ADD COLUMN internal_price REAL;
            ALTER TABLE properties ADD COLUMN internal_notes TEXT;

            CREATE INDEX idx_properties_owner_client ON properties(owner_client_id);
            CREATE INDEX idx_properties_purpose ON properties(purpose);
            CREATE INDEX idx_properties_type ON properties(property_type);
        "#,
    },
    Migration {
        version: 7,
        name: "brochures_nullable_property_and_updated_at",
        sql: r#"
            CREATE TABLE brochures_v7 (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                property_id TEXT,
                template_id TEXT,
                title TEXT,
                content TEXT,
                created_at TEXT DEFAULT (datetime('now')),
                updated_at TEXT DEFAULT (datetime('now')),
                seed_batch_id TEXT,
                FOREIGN KEY (user_id) REFERENCES users(id),
                FOREIGN KEY (property_id) REFERENCES properties(id) ON DELETE SET NULL
            );

            INSERT INTO brochures_v7 (
                id, user_id, property_id, template_id, title, content, created_at, updated_at, seed_batch_id
            )
            SELECT
                id, user_id,
                CASE WHEN property_id IS NULL OR property_id = '' THEN NULL ELSE property_id END,
                template_id, title, content, created_at, created_at, seed_batch_id
            FROM brochures;

            DROP TABLE brochures;
            ALTER TABLE brochures_v7 RENAME TO brochures;

            CREATE INDEX idx_brochures_user ON brochures(user_id);
            CREATE INDEX idx_brochures_seed_batch ON brochures(seed_batch_id);
            CREATE INDEX idx_brochures_updated ON brochures(updated_at);
        "#,
    },
    Migration {
        version: 8,
        name: "marketing_campaigns",
        sql: r#"
            CREATE TABLE marketing_campaigns (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                name TEXT NOT NULL,
                property_id TEXT,
                platform TEXT NOT NULL,
                ad_link TEXT,
                start_date TEXT,
                end_date TEXT,
                views INTEGER NOT NULL DEFAULT 0,
                inquiries INTEGER NOT NULL DEFAULT 0,
                viewings INTEGER NOT NULL DEFAULT 0,
                deals INTEGER NOT NULL DEFAULT 0,
                cost REAL NOT NULL DEFAULT 0,
                notes TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                seed_batch_id TEXT,
                FOREIGN KEY (user_id) REFERENCES users(id),
                FOREIGN KEY (property_id) REFERENCES properties(id) ON DELETE SET NULL
            );

            CREATE INDEX idx_campaigns_user ON marketing_campaigns(user_id);
            CREATE INDEX idx_campaigns_platform ON marketing_campaigns(platform);
            CREATE INDEX idx_campaigns_property ON marketing_campaigns(property_id);
            CREATE INDEX idx_campaigns_start ON marketing_campaigns(start_date);
            CREATE INDEX idx_campaigns_updated ON marketing_campaigns(updated_at);
            CREATE INDEX idx_campaigns_seed_batch ON marketing_campaigns(seed_batch_id);
        "#,
    },
    Migration {
        version: 9,
        name: "office_appointments",
        sql: r#"
            CREATE TABLE office_appointments (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                title TEXT NOT NULL,
                occurs_on TEXT NOT NULL,
                starts_at TEXT,
                notes TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                seed_batch_id TEXT,
                FOREIGN KEY (user_id) REFERENCES users(id)
            );

            CREATE INDEX idx_appointments_user ON office_appointments(user_id);
            CREATE INDEX idx_appointments_day ON office_appointments(occurs_on);
            CREATE INDEX idx_appointments_user_day ON office_appointments(user_id, occurs_on);
            CREATE INDEX idx_appointments_seed_batch ON office_appointments(seed_batch_id);
        "#,
    },
    Migration {
        version: 10,
        name: "broker_finance",
        sql: r#"
            CREATE TABLE finance_operations (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                title TEXT NOT NULL,
                occurs_on TEXT NOT NULL,
                deal_value REAL NOT NULL DEFAULT 0,
                client_id TEXT,
                property_id TEXT,
                campaign_id TEXT,
                notes TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                seed_batch_id TEXT,
                FOREIGN KEY (user_id) REFERENCES users(id),
                FOREIGN KEY (client_id) REFERENCES clients(id) ON DELETE SET NULL,
                FOREIGN KEY (property_id) REFERENCES properties(id) ON DELETE SET NULL,
                FOREIGN KEY (campaign_id) REFERENCES marketing_campaigns(id) ON DELETE SET NULL
            );

            CREATE TABLE finance_invoices (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                number TEXT NOT NULL,
                title TEXT NOT NULL,
                issue_date TEXT NOT NULL,
                due_date TEXT,
                amount REAL NOT NULL,
                client_id TEXT,
                property_id TEXT,
                campaign_id TEXT,
                operation_id TEXT,
                notes TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                seed_batch_id TEXT,
                FOREIGN KEY (user_id) REFERENCES users(id),
                FOREIGN KEY (client_id) REFERENCES clients(id) ON DELETE SET NULL,
                FOREIGN KEY (property_id) REFERENCES properties(id) ON DELETE SET NULL,
                FOREIGN KEY (campaign_id) REFERENCES marketing_campaigns(id) ON DELETE SET NULL,
                FOREIGN KEY (operation_id) REFERENCES finance_operations(id) ON DELETE SET NULL,
                UNIQUE (user_id, number)
            );

            CREATE TABLE finance_commissions (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                title TEXT NOT NULL,
                occurs_on TEXT NOT NULL,
                mode TEXT NOT NULL,
                basis_amount REAL NOT NULL DEFAULT 0,
                rate_percent REAL,
                amount_fixed REAL,
                office_share_pct REAL NOT NULL DEFAULT 0,
                broker_share_pct REAL NOT NULL DEFAULT 0,
                marketer_share_pct REAL NOT NULL DEFAULT 0,
                partner_share_pct REAL NOT NULL DEFAULT 0,
                client_id TEXT,
                property_id TEXT,
                campaign_id TEXT,
                operation_id TEXT,
                invoice_id TEXT,
                notes TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                seed_batch_id TEXT,
                FOREIGN KEY (user_id) REFERENCES users(id),
                FOREIGN KEY (client_id) REFERENCES clients(id) ON DELETE SET NULL,
                FOREIGN KEY (property_id) REFERENCES properties(id) ON DELETE SET NULL,
                FOREIGN KEY (campaign_id) REFERENCES marketing_campaigns(id) ON DELETE SET NULL,
                FOREIGN KEY (operation_id) REFERENCES finance_operations(id) ON DELETE SET NULL,
                FOREIGN KEY (invoice_id) REFERENCES finance_invoices(id) ON DELETE SET NULL
            );

            CREATE TABLE finance_expenses (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                title TEXT NOT NULL,
                occurs_on TEXT NOT NULL,
                amount REAL NOT NULL,
                category TEXT,
                client_id TEXT,
                property_id TEXT,
                campaign_id TEXT,
                reference TEXT,
                notes TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                seed_batch_id TEXT,
                FOREIGN KEY (user_id) REFERENCES users(id),
                FOREIGN KEY (client_id) REFERENCES clients(id) ON DELETE SET NULL,
                FOREIGN KEY (property_id) REFERENCES properties(id) ON DELETE SET NULL,
                FOREIGN KEY (campaign_id) REFERENCES marketing_campaigns(id) ON DELETE SET NULL
            );

            CREATE TABLE finance_payments (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                occurs_on TEXT NOT NULL,
                amount REAL NOT NULL,
                target_type TEXT NOT NULL,
                target_id TEXT NOT NULL,
                method TEXT,
                reference TEXT,
                notes TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                seed_batch_id TEXT,
                FOREIGN KEY (user_id) REFERENCES users(id)
            );

            CREATE INDEX idx_fin_ops_user ON finance_operations(user_id);
            CREATE INDEX idx_fin_ops_day ON finance_operations(occurs_on);
            CREATE INDEX idx_fin_inv_user ON finance_invoices(user_id);
            CREATE INDEX idx_fin_inv_day ON finance_invoices(issue_date);
            CREATE INDEX idx_fin_inv_number ON finance_invoices(user_id, number);
            CREATE INDEX idx_fin_com_user ON finance_commissions(user_id);
            CREATE INDEX idx_fin_com_day ON finance_commissions(occurs_on);
            CREATE INDEX idx_fin_exp_user ON finance_expenses(user_id);
            CREATE INDEX idx_fin_exp_day ON finance_expenses(occurs_on);
            CREATE INDEX idx_fin_pay_user ON finance_payments(user_id);
            CREATE INDEX idx_fin_pay_target ON finance_payments(user_id, target_type, target_id);
            CREATE INDEX idx_fin_pay_day ON finance_payments(occurs_on);
        "#,
    },
    Migration {
        version: 11,
        name: "ui_locale_prefs",
        sql: r#"
            ALTER TABLE settings ADD COLUMN ui_language TEXT;
            ALTER TABLE settings ADD COLUMN display_currency TEXT;
        "#,
    },
    Migration {
        version: 12,
        name: "campaign_link_tracking",
        sql: r#"
            ALTER TABLE marketing_campaigns ADD COLUMN original_ad_url TEXT;
            ALTER TABLE marketing_campaigns ADD COLUMN tracking_code TEXT;
            ALTER TABLE marketing_campaigns ADD COLUMN tracking_url TEXT;
            ALTER TABLE marketing_campaigns ADD COLUMN tracked_clicks INTEGER NOT NULL DEFAULT 0;
            ALTER TABLE marketing_campaigns ADD COLUMN unique_clicks INTEGER NOT NULL DEFAULT 0;
            ALTER TABLE marketing_campaigns ADD COLUMN last_click_at TEXT;
            ALTER TABLE marketing_campaigns ADD COLUMN tracking_enabled INTEGER NOT NULL DEFAULT 0;
            CREATE UNIQUE INDEX IF NOT EXISTS idx_campaigns_tracking_code
              ON marketing_campaigns(tracking_code)
              WHERE tracking_code IS NOT NULL AND tracking_code != '';
            ALTER TABLE settings ADD COLUMN tracking_endpoint_url TEXT;
        "#,
    },
    Migration {
        version: 13,
        name: "official_training_seed_v1",
        sql: r#"
            ALTER TABLE properties ADD COLUMN is_training INTEGER NOT NULL DEFAULT 0;
            ALTER TABLE property_images ADD COLUMN is_training INTEGER NOT NULL DEFAULT 0;
            ALTER TABLE property_updates ADD COLUMN is_training INTEGER NOT NULL DEFAULT 0;
            ALTER TABLE clients ADD COLUMN is_training INTEGER NOT NULL DEFAULT 0;
            ALTER TABLE communications ADD COLUMN is_training INTEGER NOT NULL DEFAULT 0;
            ALTER TABLE client_property_links ADD COLUMN is_training INTEGER NOT NULL DEFAULT 0;
            ALTER TABLE market_reference_data ADD COLUMN is_training INTEGER NOT NULL DEFAULT 0;
            ALTER TABLE brochures ADD COLUMN is_training INTEGER NOT NULL DEFAULT 0;
            ALTER TABLE audit_logs ADD COLUMN is_training INTEGER NOT NULL DEFAULT 0;
            ALTER TABLE client_roles ADD COLUMN is_training INTEGER NOT NULL DEFAULT 0;
            ALTER TABLE marketing_campaigns ADD COLUMN is_training INTEGER NOT NULL DEFAULT 0;
            ALTER TABLE office_appointments ADD COLUMN is_training INTEGER NOT NULL DEFAULT 0;
            ALTER TABLE finance_operations ADD COLUMN is_training INTEGER NOT NULL DEFAULT 0;
            ALTER TABLE finance_invoices ADD COLUMN is_training INTEGER NOT NULL DEFAULT 0;
            ALTER TABLE finance_commissions ADD COLUMN is_training INTEGER NOT NULL DEFAULT 0;
            ALTER TABLE finance_expenses ADD COLUMN is_training INTEGER NOT NULL DEFAULT 0;
            ALTER TABLE finance_payments ADD COLUMN is_training INTEGER NOT NULL DEFAULT 0;
            ALTER TABLE file_assets ADD COLUMN seed_batch_id TEXT;
            ALTER TABLE file_assets ADD COLUMN is_training INTEGER NOT NULL DEFAULT 0;

            UPDATE properties SET seed_batch_id = 'obr-training-v1', is_training = 1 WHERE seed_batch_id = 'obr-demo-v1';
            UPDATE property_images SET seed_batch_id = 'obr-training-v1', is_training = 1 WHERE seed_batch_id = 'obr-demo-v1';
            UPDATE property_updates SET seed_batch_id = 'obr-training-v1', is_training = 1 WHERE seed_batch_id = 'obr-demo-v1';
            UPDATE clients SET seed_batch_id = 'obr-training-v1', is_training = 1 WHERE seed_batch_id = 'obr-demo-v1';
            UPDATE communications SET seed_batch_id = 'obr-training-v1', is_training = 1 WHERE seed_batch_id = 'obr-demo-v1';
            UPDATE client_property_links SET seed_batch_id = 'obr-training-v1', is_training = 1 WHERE seed_batch_id = 'obr-demo-v1';
            UPDATE market_reference_data SET seed_batch_id = 'obr-training-v1', is_training = 1 WHERE seed_batch_id = 'obr-demo-v1';
            UPDATE brochures SET seed_batch_id = 'obr-training-v1', is_training = 1 WHERE seed_batch_id = 'obr-demo-v1';
            UPDATE audit_logs SET seed_batch_id = 'obr-training-v1', is_training = 1 WHERE seed_batch_id = 'obr-demo-v1';
            UPDATE client_roles SET seed_batch_id = 'obr-training-v1', is_training = 1 WHERE seed_batch_id = 'obr-demo-v1';
            UPDATE marketing_campaigns SET seed_batch_id = 'obr-training-v1', is_training = 1 WHERE seed_batch_id = 'obr-demo-v1';
            UPDATE office_appointments SET seed_batch_id = 'obr-training-v1', is_training = 1 WHERE seed_batch_id = 'obr-demo-v1';
            UPDATE finance_operations SET seed_batch_id = 'obr-training-v1', is_training = 1 WHERE seed_batch_id = 'obr-demo-v1';
            UPDATE finance_invoices SET seed_batch_id = 'obr-training-v1', is_training = 1 WHERE seed_batch_id = 'obr-demo-v1';
            UPDATE finance_commissions SET seed_batch_id = 'obr-training-v1', is_training = 1 WHERE seed_batch_id = 'obr-demo-v1';
            UPDATE finance_expenses SET seed_batch_id = 'obr-training-v1', is_training = 1 WHERE seed_batch_id = 'obr-demo-v1';
            UPDATE finance_payments SET seed_batch_id = 'obr-training-v1', is_training = 1 WHERE seed_batch_id = 'obr-demo-v1';
            UPDATE seed_batches SET id = 'obr-training-v1', label = 'بيانات تدريب OBR الرسمية' WHERE id = 'obr-demo-v1';

            INSERT OR IGNORE INTO app_meta(key, value, updated_at) VALUES ('training_seed_deleted', 'false', datetime('now'));
        "#,
    },
    Migration {
        version: 14,
        name: "contracts_manual_provider_foundation",
        sql: r#"
            CREATE TABLE contracts (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                contract_number TEXT,
                contract_type TEXT,
                contract_scope TEXT,
                parties_json TEXT NOT NULL DEFAULT '[]',
                property_id TEXT,
                client_id TEXT,
                campaign_id TEXT,
                commission_id TEXT,
                invoice_id TEXT,
                start_date TEXT,
                end_date TEXT,
                status TEXT NOT NULL DEFAULT 'draft',
                commission_amount REAL,
                commission_mechanism TEXT,
                license_number TEXT,
                pdf_asset_id TEXT,
                source TEXT NOT NULL DEFAULT 'manual_draft',
                provider_fields_json TEXT NOT NULL DEFAULT '{}',
                extracted_text_excerpt TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (user_id) REFERENCES users(id),
                FOREIGN KEY (property_id) REFERENCES properties(id) ON DELETE SET NULL,
                FOREIGN KEY (client_id) REFERENCES clients(id) ON DELETE SET NULL,
                FOREIGN KEY (campaign_id) REFERENCES marketing_campaigns(id) ON DELETE SET NULL,
                FOREIGN KEY (commission_id) REFERENCES finance_commissions(id) ON DELETE SET NULL,
                FOREIGN KEY (invoice_id) REFERENCES finance_invoices(id) ON DELETE SET NULL,
                FOREIGN KEY (pdf_asset_id) REFERENCES file_assets(id) ON DELETE SET NULL
            );
            CREATE INDEX idx_contracts_user_status ON contracts(user_id, status);
            CREATE INDEX idx_contracts_user_end_date ON contracts(user_id, end_date);
        "#,
    },
    Migration { version: 15, name: "country_record_context", sql: "" },
    Migration { version: 16, name: "official_integrations", sql: "CREATE TABLE official_integrations (id TEXT PRIMARY KEY, user_id TEXT NOT NULL REFERENCES users(id), config TEXT NOT NULL, secrets BLOB NOT NULL); CREATE INDEX official_integrations_user ON official_integrations(user_id); CREATE UNIQUE INDEX official_integrations_active ON official_integrations(user_id,json_extract(config,'$.country'),json_extract(config,'$.jurisdiction')) WHERE json_extract(config,'$.enabled')=1;" },
    Migration { version: 17, name: "capacity_indexes", sql: r#"
            CREATE INDEX IF NOT EXISTS idx_clients_user_created ON clients(user_id, created_at);
            CREATE INDEX IF NOT EXISTS idx_clients_user_city_created ON clients(user_id, city, created_at);
            CREATE INDEX IF NOT EXISTS idx_clients_user_status ON clients(user_id, status);
            CREATE INDEX IF NOT EXISTS idx_clients_user_reltype ON clients(user_id, relationship_type);
            CREATE INDEX IF NOT EXISTS idx_market_user_entry ON market_reference_data(user_id, entry_date);
            CREATE INDEX IF NOT EXISTS idx_market_user_city_entry ON market_reference_data(user_id, city, entry_date);
            CREATE INDEX IF NOT EXISTS idx_props_user_updated ON properties(user_id, updated_at);
            CREATE INDEX IF NOT EXISTS idx_props_user_city_updated ON properties(user_id, city, updated_at);
            CREATE INDEX IF NOT EXISTS idx_props_user_purpose ON properties(user_id, purpose);
            CREATE INDEX IF NOT EXISTS idx_props_user_status ON properties(user_id, status);
        "# },
];

fn table_exists(conn: &Connection, name: &str) -> Result<bool, String> {
    let mut stmt = conn
        .prepare("SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1 LIMIT 1")
        .map_err(|e| e.to_string())?;
    let exists = stmt.exists([name]).map_err(|e| e.to_string())?;
    Ok(exists)
}

fn ensure_migrations_table(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        "#,
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn current_version(conn: &Connection) -> Result<i32, String> {
    let v: i32 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);
    Ok(v)
}

fn stamp_migration(conn: &Connection, version: i32, name: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO schema_migrations(version, name, applied_at) VALUES (?1, ?2, datetime('now'))",
        rusqlite::params![version, name],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// يشغّل Migrations داخل معاملات. يتعامل مع قواعد قديمة وُلدت قبل النظام الإصداري.
pub fn run_migrations(conn: &mut Connection) -> Result<i32, String> {
    conn.pragma_update(None, "foreign_keys", "ON")
        .map_err(|e| e.to_string())?;

    ensure_migrations_table(conn)?;

    let mut version = current_version(conn)?;

    // ترقية قواعد قديمة: إن وُجدت جداول الإنتاج دون سجل migrations، نعتبر V1 مطبّقة.
    if version == 0 && table_exists(conn, "users")? {
        stamp_migration(conn, 1, "baseline_core_tables_preexisting")?;
        version = 1;
    }

    for m in MIGRATIONS {
        if m.version <= version {
            continue;
        }

        // قواعد قديمة قد تكون وصلت لـ V1 عبر CREATE IF NOT EXISTS السابق
        // ثم نطبّق V2+ فقط.
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        tx.execute_batch(m.sql)
            .map_err(|e| format!("فشل Migration {} ({}): {}", m.version, m.name, e))?;
        if m.version == 15 { crate::country::migrate(&tx)?; }
        tx.execute(
            "INSERT INTO schema_migrations(version, name, applied_at) VALUES (?1, ?2, datetime('now'))",
            rusqlite::params![m.version, m.name],
        )
        .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        version = m.version;
    }

    // فحص سلامة سريع
    let ok: String = conn
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;
    if ok != "ok" {
        return Err(format!("فشل integrity_check: {ok}"));
    }

    Ok(version)
}

pub fn schema_version(conn: &Connection) -> Result<i32, String> {
    ensure_migrations_table(conn)?;
    current_version(conn)
}
