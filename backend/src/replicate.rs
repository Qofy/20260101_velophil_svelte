use anyhow::{anyhow, Result};
use deadpool_postgres::{Manager, ManagerConfig, Pool, RecyclingMethod};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use tokio::sync::RwLock;
use tokio_postgres::{Config as PgConfig, NoTls};

#[derive(Clone)]
pub struct Replicator {
    pools: Vec<Pool>,
    routes: HashMap<String, Vec<usize>>, // table -> pool indexes
    ensured: Arc<RwLock<HashSet<(usize, String)>>>, // (pool_index, table) ensured
}

impl Replicator {
    pub async fn new(conn_strings: &[String], routes: HashMap<String, Vec<usize>>) -> Result<Self> {
        let mut pools = Vec::new();
        for cs in conn_strings {
            let cfg: PgConfig = cs
                .parse()
                .map_err(|e| anyhow!("invalid pg conn string: {e}"))?;
            let mgr = Manager::from_config(
                cfg,
                NoTls,
                ManagerConfig {
                    recycling_method: RecyclingMethod::Fast,
                },
            );
            let pool = Pool::builder(mgr).max_size(8).build().unwrap();
            pools.push(pool);
        }
        Ok(Self {
            pools,
            routes,
            ensured: Arc::new(RwLock::new(HashSet::new())),
        })
    }

    async fn ensure_table(&self, pool_idx: usize, table: &str) -> Result<()> {
        let key = (pool_idx, table.to_string());
        {
            let ensured = self.ensured.read().await;
            if ensured.contains(&key) {
                return Ok(());
            }
        }
        let client = self.pools[pool_idx].get().await?;
        let sql = format!(
            "CREATE TABLE IF NOT EXISTS quoteflow_{} (id TEXT PRIMARY KEY, last_updated TIMESTAMPTZ, data JSONB)",
            table
        );
        client.batch_execute(&sql).await?;
        let mut ensured = self.ensured.write().await;
        ensured.insert(key);
        Ok(())
    }

    pub async fn upsert(
        &self,
        table: &str,
        id: &str,
        last_updated: &str,
        json: &serde_json::Value,
    ) -> Result<()> {
        if self.pools.is_empty() {
            return Ok(());
        }
        let Some(targets) = self.routes.get(table) else {
            return Ok(());
        };
        let data = serde_json::to_value(json)?; // ensure value
        for &i in targets.iter() {
            self.ensure_table(i, table).await.ok();
            let client = self.pools[i].get().await?;
            let sql = format!(
                "INSERT INTO quoteflow_{} (id, last_updated, data) VALUES ($1, $2, $3)
                 ON CONFLICT (id) DO UPDATE SET last_updated = EXCLUDED.last_updated, data = EXCLUDED.data",
                table
            );
            let ts = if last_updated.is_empty() {
                None
            } else {
                chrono::DateTime::parse_from_rfc3339(last_updated)
                    .ok()
                    .map(|d| d.with_timezone(&chrono::Utc))
            };
            let _ = client.execute(&sql, &[&id, &ts, &data]).await?;
        }
        Ok(())
    }

    pub async fn delete(&self, table: &str, id: &str) -> Result<()> {
        if self.pools.is_empty() {
            return Ok(());
        }
        let Some(targets) = self.routes.get(table) else {
            return Ok(());
        };
        for &i in targets.iter() {
            self.ensure_table(i, table).await.ok();
            let client = self.pools[i].get().await?;
            let sql = format!("DELETE FROM quoteflow_{} WHERE id = $1", table);
            let _ = client.execute(&sql, &[&id]).await?;
        }
        Ok(())
    }
}
