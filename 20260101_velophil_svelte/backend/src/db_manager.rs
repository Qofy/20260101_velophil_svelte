use anyhow::{anyhow, Result};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
};
use tokio_postgres::NoTls;

pub struct DbManager {
    pub url: String,
}

impl DbManager {
    pub fn new(url: String) -> Self {
        Self { url }
    }

    pub async fn test_connection(&self) -> Result<()> {
        let (_client, connection) = tokio_postgres::connect(&self.url, NoTls).await?;
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });
        Ok(())
    }

    async fn ensure_database_exists(&self) -> Result<()> {
        // Try connecting to target DB; if it works, return Ok
        if tokio_postgres::connect(&self.url, NoTls).await.is_ok() {
            return Ok(());
        }
        // Parse URL into Config and target db name
        let mut cfg: tokio_postgres::Config = self
            .url
            .parse()
            .map_err(|e| anyhow!("invalid connection string: {}", e))?;
        let target_db = cfg.get_dbname().unwrap_or("postgres").to_string();
        // Use admin connection to 'postgres' database on same host/user
        cfg.dbname("postgres");
        let (client, connection) = cfg
            .connect(NoTls)
            .await
            .map_err(|e| anyhow!("admin connect failed: {}", e))?;
        tokio::spawn(async move {
            let _ = connection.await;
        });
        // Check if database exists
        let exists = client
            .query_one(
                "SELECT 1 FROM pg_database WHERE datname = $1",
                &[&target_db],
            )
            .await
            .is_ok();
        if exists {
            return Ok(());
        }
        // Validate identifier (simple safe rule: letters, digits, underscore only)
        if !target_db
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Err(anyhow!(
                "refusing to create database with unsafe name: {}",
                target_db
            ));
        }
        let create_sql = format!("CREATE DATABASE {}", target_db);
        client
            .batch_execute(&create_sql)
            .await
            .map_err(|e| anyhow!("create database failed: {}", e))?;
        Ok(())
    }

    pub async fn create_schema(&self, include_extensions: bool) -> Result<()> {
        self.ensure_database_exists().await?;
        let sql = self.generate_initial_sql(false).await?;
        let (client, connection) = tokio_postgres::connect(&self.url, NoTls).await?;
        tokio::spawn(async move {
            let _ = connection.await;
        });
        if include_extensions {
            client
                .batch_execute("CREATE EXTENSION IF NOT EXISTS pgcrypto;")
                .await?;
        }
        client.batch_execute(&sql).await?;
        Ok(())
    }

    pub async fn generate_initial_sql(&self, include_sample_data: bool) -> Result<String> {
        let mut out = String::new();
        out.push_str("-- QuoteFlow initial schema\n");
        out.push_str("CREATE EXTENSION IF NOT EXISTS pgcrypto;\n\n");

        // Core tables based on backend models
        out.push_str(&Self::sql_core_tables());

        // Additional tables from JSON entities (../src/entities/*.json)
        if let Ok(sql) = Self::sql_from_json_entities() {
            out.push_str(&sql);
        }

        if include_sample_data {
            out.push_str(&Self::sql_sample_data());
        }
        Ok(out)
    }

    pub async fn seed_sample_data(&self) -> Result<()> {
        self.ensure_database_exists().await?;
        let (client, connection) = tokio_postgres::connect(&self.url, NoTls).await?;
        tokio::spawn(async move {
            let _ = connection.await;
        });
        // Ensure schema exists
        client
            .batch_execute("CREATE EXTENSION IF NOT EXISTS pgcrypto;")
            .await?;
        client.batch_execute(&Self::sql_core_tables()).await?;
        // Insert sample data
        client.batch_execute(&Self::sql_sample_data()).await?;
        Ok(())
    }

    pub async fn dump(
        &self,
        output: String,
        include_schema: bool,
        include_data: bool,
        tables: Option<String>,
    ) -> Result<()> {
        // Prefer pg_dump if available
        if which::which("pg_dump").is_ok() {
            let mut args: Vec<String> = vec![self.url.clone(), "-f".into(), output.clone()];
            if include_schema && !include_data {
                args.push("-s".into());
            }
            if include_data && !include_schema {
                args.push("-a".into());
            }
            if let Some(tables_csv) = tables {
                for t in tables_csv.split(',') {
                    let t = t.trim();
                    if !t.is_empty() {
                        args.push("-t".into());
                        args.push(t.into());
                    }
                }
            }
            // Use env var PGPASSWORD from URL if any is not handled, so we rely on URL
            let status = std::process::Command::new("pg_dump").args(args).status()?;
            if !status.success() {
                return Err(anyhow!("pg_dump failed"));
            }
            return Ok(());
        }
        // Fallback: simple export (schema only)
        let sql = self.generate_initial_sql(false).await?;
        fs::write(&output, sql)?;
        Ok(())
    }

    pub async fn import(&self, input: String, drop_existing: bool) -> Result<()> {
        self.ensure_database_exists().await?;
        let sql = fs::read_to_string(&input)?;
        let (client, connection) = tokio_postgres::connect(&self.url, NoTls).await?;
        tokio::spawn(async move {
            let _ = connection.await;
        });
        if drop_existing {
            // Best-effort drop known tables
            client
                .batch_execute(Self::sql_drop_known_tables().as_str())
                .await
                .ok();
        }
        client.batch_execute(&sql).await?;
        Ok(())
    }

    pub async fn reset(&self) -> Result<()> {
        let (client, connection) = tokio_postgres::connect(&self.url, NoTls).await?;
        tokio::spawn(async move {
            let _ = connection.await;
        });
        client
            .batch_execute(Self::sql_drop_known_tables().as_str())
            .await?;
        Ok(())
    }

    fn sql_drop_known_tables() -> String {
        let tables = vec![
            // core
            "invoices",
            "quotes",
            "customers",
            "certificates",
            // common from entities
            "appointments",
            "products",
            "companies",
            "quote_comments",
            "intern_certificates",
            "books",
            "blogs",
            "galleries",
            "blog_categories",
            "blog_tags",
            "reviews",
            "book_comments",
            "contact_messages",
        ];
        let mut sql = String::new();
        for t in tables {
            sql.push_str(&format!("DROP TABLE IF EXISTS {} CASCADE;\n", t));
        }
        sql
    }

    fn sql_core_tables() -> String {
        let mut s = String::new();
        s.push_str(
            "CREATE TABLE IF NOT EXISTS customers (\n\
             id UUID PRIMARY KEY DEFAULT gen_random_uuid(),\n\
             name TEXT NOT NULL,\n\
             email TEXT NOT NULL,\n\
             phone TEXT NOT NULL,\n\
             address JSONB NOT NULL,\n\
             contact_person TEXT,\n\
             notes TEXT,\n\
             created_date TIMESTAMPTZ NOT NULL DEFAULT now(),\n\
             last_updated TIMESTAMPTZ NOT NULL DEFAULT now()\n\
            );\n\
            CREATE INDEX IF NOT EXISTS idx_customers_email ON customers(email);\n\
            CREATE INDEX IF NOT EXISTS idx_customers_name ON customers(name);\n\n",
        );

        s.push_str(
            "CREATE TABLE IF NOT EXISTS quotes (\n\
             id UUID PRIMARY KEY DEFAULT gen_random_uuid(),\n\
             quote_number TEXT NOT NULL UNIQUE,\n\
             company_id TEXT,\n\
             customer_id UUID,\n\
             customer_name TEXT,\n\
             customer_email TEXT,\n\
             title TEXT NOT NULL,\n\
             status TEXT NOT NULL DEFAULT 'draft',\n\
             public_view_enabled BOOLEAN NOT NULL DEFAULT true,\n\
             valid_until TIMESTAMPTZ,\n\
             approval_token TEXT,\n\
             approved_date TIMESTAMPTZ,\n\
             approved_by TEXT,\n\
             rejected_date TIMESTAMPTZ,\n\
             rejected_by TEXT,\n\
             rejection_reason TEXT,\n\
             items JSONB NOT NULL,\n\
             attachments JSONB NOT NULL,\n\
             reference_url TEXT,\n\
             subtotal NUMERIC(12,2) NOT NULL DEFAULT 0,\n\
             tax_rate NUMERIC(6,3) NOT NULL DEFAULT 0,\n\
             tax_amount NUMERIC(12,2) NOT NULL DEFAULT 0,\n\
             total_amount NUMERIC(12,2) NOT NULL DEFAULT 0,\n\
             notes TEXT,\n\
             converted_to_invoice BOOLEAN NOT NULL DEFAULT false,\n\
             converted_invoice_id UUID,\n\
             created_date TIMESTAMPTZ NOT NULL DEFAULT now(),\n\
             last_updated TIMESTAMPTZ NOT NULL DEFAULT now(),\n\
             CONSTRAINT fk_quotes_customer FOREIGN KEY(customer_id) REFERENCES customers(id)\n\
            );\n\
            CREATE INDEX IF NOT EXISTS idx_quotes_status ON quotes(status);\n\
            CREATE INDEX IF NOT EXISTS idx_quotes_customer_id ON quotes(customer_id);\n\n",
        );

        s.push_str(
            "CREATE TABLE IF NOT EXISTS invoices (\n\
             id UUID PRIMARY KEY DEFAULT gen_random_uuid(),\n\
             invoice_number TEXT NOT NULL UNIQUE,\n\
             company_id TEXT,\n\
             quote_id UUID,\n\
             customer_id UUID,\n\
             customer_name TEXT,\n\
             title TEXT NOT NULL,\n\
             status TEXT NOT NULL DEFAULT 'draft',\n\
             items JSONB NOT NULL,\n\
             subtotal NUMERIC(12,2) NOT NULL DEFAULT 0,\n\
             tax_rate NUMERIC(6,3) NOT NULL DEFAULT 0,\n\
             tax_amount NUMERIC(12,2) NOT NULL DEFAULT 0,\n\
             total_amount NUMERIC(12,2) NOT NULL DEFAULT 0,\n\
             paid_amount NUMERIC(12,2) NOT NULL DEFAULT 0,\n\
             payment_date TIMESTAMPTZ,\n\
             due_date TIMESTAMPTZ,\n\
             notes TEXT,\n\
             created_date TIMESTAMPTZ NOT NULL DEFAULT now(),\n\
             last_updated TIMESTAMPTZ NOT NULL DEFAULT now(),\n\
             CONSTRAINT fk_invoices_customer FOREIGN KEY(customer_id) REFERENCES customers(id),\n\
             CONSTRAINT fk_invoices_quote FOREIGN KEY(quote_id) REFERENCES quotes(id)\n\
            );\n\
            CREATE INDEX IF NOT EXISTS idx_invoices_status ON invoices(status);\n\
            CREATE INDEX IF NOT EXISTS idx_invoices_customer_id ON invoices(customer_id);\n\n",
        );

        s.push_str(
            "CREATE TABLE IF NOT EXISTS certificates (\n\
             id UUID PRIMARY KEY DEFAULT gen_random_uuid(),\n\
             certificate_number TEXT NOT NULL UNIQUE,\n\
             student_name TEXT NOT NULL,\n\
             company_name TEXT NOT NULL,\n\
             total_hours NUMERIC(10,2) NOT NULL DEFAULT 0,\n\
             start_date DATE NOT NULL,\n\
             end_date DATE NOT NULL,\n\
             tasks_description TEXT NOT NULL,\n\
             company_logo_url TEXT,\n\
             supervisor_name TEXT,\n\
             supervisor_title TEXT,\n\
             supervisor_signature_url TEXT,\n\
             created_date TIMESTAMPTZ NOT NULL DEFAULT now(),\n\
             last_updated TIMESTAMPTZ NOT NULL DEFAULT now()\n\
            );\n\
            CREATE INDEX IF NOT EXISTS idx_certificates_student ON certificates(student_name);\n\
            CREATE INDEX IF NOT EXISTS idx_certificates_company ON certificates(company_name);\n\n",
        );

        s
    }

    fn sql_sample_data() -> String {
        let mut s = String::new();
        s.push_str("-- sample customers\n");
        s.push_str("INSERT INTO customers (id,name,email,phone,address,contact_person,notes) VALUES \
            (gen_random_uuid(),'Globex Corporation','contact@globex.test','+1-555-1000','{\"street\":\"100 Market St\",\"city\":\"Springfield\",\"state\":\"IL\",\"zip\":\"62701\",\"country\":\"USA\"}','Hank Scorpio','VIP customer') \
            ON CONFLICT DO NOTHING;\n");
        s.push_str("INSERT INTO customers (id,name,email,phone,address,contact_person,notes) VALUES \
            (gen_random_uuid(),'Wayne Enterprises','info@wayne.test','+1-555-2000','{\"street\":\"1 Wayne Tower\",\"city\":\"Gotham\",\"state\":\"NJ\",\"zip\":\"07097\",\"country\":\"USA\"}','Bruce Wayne',NULL) \
            ON CONFLICT DO NOTHING;\n");
        s.push_str("INSERT INTO customers (id,name,email,phone,address,contact_person,notes) VALUES \
            (gen_random_uuid(),'Stark Industries','sales@stark.test','+1-555-3000','{\"street\":\"200 Park Ave\",\"city\":\"New York\",\"state\":\"NY\",\"zip\":\"10166\",\"country\":\"USA\"}','Tony Stark',NULL) \
            ON CONFLICT DO NOTHING;\n");

        s.push_str("-- sample quote\n");
        s.push_str("WITH c AS (SELECT id FROM customers ORDER BY created_date LIMIT 1)
            INSERT INTO quotes (id,quote_number,customer_id,customer_name,customer_email,title,status,public_view_enabled,items,attachments,subtotal,tax_rate,tax_amount,total_amount,notes)
            SELECT gen_random_uuid(),'QT-1001',id,'Globex Corporation','contact@globex.test','Website Redesign','draft',true,'[]'::jsonb,'[]'::jsonb,0,19,0,0,'Initial scope' FROM c \
            ON CONFLICT DO NOTHING;\n");

        s.push_str("-- sample invoice\n");
        s.push_str("WITH c AS (SELECT id FROM customers ORDER BY created_date OFFSET 1 LIMIT 1)
            INSERT INTO invoices (id,invoice_number,customer_id,customer_name,title,status,items,subtotal,tax_rate,tax_amount,total_amount,paid_amount,notes)
            SELECT gen_random_uuid(),'INV-1001',id,'Wayne Enterprises','Q1 Consulting','sent','[]'::jsonb,0,19,0,0,0,'Net 30' FROM c \
            ON CONFLICT DO NOTHING;\n");

        s.push_str("-- sample certificate\n");
        s.push_str("INSERT INTO certificates (id,certificate_number,student_name,company_name,total_hours,start_date,end_date,tasks_description,supervisor_name,supervisor_title) VALUES \
            (gen_random_uuid(),'CERT-0001','Jane Doe','Acme Corp',120,'2025-07-01','2025-08-15','Worked on various tasks','John Manager','CTO') \
            ON CONFLICT DO NOTHING;\n");

        s
    }

    fn sql_from_json_entities() -> Result<String> {
        let dir = Self::entities_dir();
        if !dir.exists() {
            return Ok(String::new());
        }
        let mut out = String::new();
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let sql = Self::table_sql_from_json(&path)?;
            out.push_str(&sql);
        }
        Ok(out)
    }

    fn entities_dir() -> PathBuf {
        // Try ../src/entities relative to backend
        Path::new("../src/entities").to_path_buf()
    }

    fn table_sql_from_json(path: &Path) -> Result<String> {
        let content = fs::read_to_string(path)?;
        let v: Value = serde_json::from_str(&content)?;
        let table = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        let table = Self::pluralize_snake(table);
        let mut cols: Vec<(String, String, bool)> = Vec::new(); // name, type, not_null
        cols.push((
            "id".into(),
            "UUID PRIMARY KEY DEFAULT gen_random_uuid()".into(),
            true,
        ));
        if let Some(props) = v.get("properties").and_then(|p| p.as_object()) {
            let required: Vec<String> = v
                .get("required")
                .and_then(|r| r.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|x| x.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            for (name, schema) in props.iter() {
                let (ty, nn) = Self::pg_type_from_json_schema(name, schema, &required);
                cols.push((name.clone(), ty, nn));
            }
        }
        // timestamps
        cols.push((
            "created_at".into(),
            "TIMESTAMPTZ NOT NULL DEFAULT now()".into(),
            true,
        ));
        cols.push((
            "updated_at".into(),
            "TIMESTAMPTZ NOT NULL DEFAULT now()".into(),
            true,
        ));

        let mut sql = format!("CREATE TABLE IF NOT EXISTS {} (\n", table);
        for (i, (n, t, _)) in cols.iter().enumerate() {
            sql.push_str(&format!(
                "  {}{}{}\n",
                n,
                if t.is_empty() {
                    String::new()
                } else {
                    format!(" {}", t)
                },
                if i + 1 == cols.len() {
                    String::new()
                } else {
                    ",".into()
                }
            ));
        }
        sql.push_str(");\n\n");

        // basic indexes for *_id
        for (n, _t, _nn) in cols.iter() {
            if n.ends_with("_id") && n != "id" {
                sql.push_str(&format!(
                    "CREATE INDEX IF NOT EXISTS idx_{}_{} ON {}({});\n",
                    table, n, table, n
                ));
            }
        }
        sql.push('\n');
        Ok(sql)
    }

    fn pg_type_from_json_schema(name: &str, schema: &Value, required: &[String]) -> (String, bool) {
        let ty = schema
            .get("type")
            .and_then(|t| t.as_str())
            .unwrap_or("string");
        let fmt = schema.get("format").and_then(|f| f.as_str()).unwrap_or("");
        let not_null = required.iter().any(|r| r == name);
        let base: String = match (ty, fmt) {
            ("string", "date") => "DATE".into(),
            ("string", "date-time") => "TIMESTAMPTZ".into(),
            ("string", _) => "TEXT".into(),
            ("integer", _) => "INTEGER".into(),
            ("number", _) => "DOUBLE PRECISION".into(),
            ("boolean", _) => "BOOLEAN".into(),
            ("array", _) => "JSONB".into(),
            ("object", _) => "JSONB".into(),
            _ => "TEXT".into(),
        };
        let mut ty_full = base;
        if not_null {
            ty_full.push_str(" NOT NULL");
        }
        (ty_full, not_null)
    }

    fn pluralize_snake(name: &str) -> String {
        let snake = Self::to_snake_case(name);
        if snake.ends_with('y') {
            format!("{}ies", &snake[..snake.len() - 1])
        } else if snake.ends_with('s') {
            snake
        } else {
            format!("{}s", snake)
        }
    }

    fn to_snake_case(name: &str) -> String {
        let mut out = String::new();
        for (i, c) in name.chars().enumerate() {
            if c.is_uppercase() {
                if i > 0 {
                    out.push('_');
                }
                out.push(c.to_ascii_lowercase());
            } else {
                out.push(c);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_generate_sql_contains_core_tables() {
        let mgr = DbManager::new("postgres://user:pass@localhost/db".into());
        let sql = mgr.generate_initial_sql(false).await.unwrap();
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS customers"));
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS quotes"));
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS invoices"));
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS certificates"));
    }
}
