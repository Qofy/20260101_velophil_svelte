
// ============================================================================
// src/db.rs - FIXED UPDATE METHOD
// ============================================================================
use anyhow::Result;
use serde::{de::DeserializeOwned, Serialize};
use sled::Db;
use std::sync::Arc;
use crate::replicate::Replicator;

#[derive(Clone)]
pub struct Database {
    pub db: Arc<Db>,
    replicator: Option<Arc<Replicator>>,
}

impl Database {
    pub fn new(path: &str) -> Result<Self> {
        let db = sled::open(path)?;
        Ok(Self {
            db: Arc::new(db),
            replicator: None,
        })
    }

    pub fn with_replicator(mut self, replicator: Option<Arc<Replicator>>) -> Self {
        self.replicator = replicator;
        self
    }

    pub fn insert<T: Serialize>(&self, collection: &str, key: &str, value: &T) -> Result<()> {
        let tree = self.db.open_tree(collection)?;
        let serialized = serde_json::to_vec(value)?;
        tree.insert(key, serialized.clone())?;
        self.db.flush()?;

        if let Some(rep) = &self.replicator {
            let table = collection.to_string();
            let id = key.to_string();
            let json_value: serde_json::Value =
                serde_json::from_slice(&serialized).unwrap_or(serde_json::json!({}));
            let last_updated = json_value
                .get("last_updated")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let rep = rep.clone();
            tokio::spawn(async move {
                let _ = rep.upsert(&table, &id, &last_updated, &json_value).await;
            });
        }
        Ok(())
    }

    pub fn get<T: DeserializeOwned>(&self, collection: &str, key: &str) -> Result<Option<T>> {
        let tree = self.db.open_tree(collection)?;
        if let Some(data) = tree.get(key)? {
            let value: T = serde_json::from_slice(&data)?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    pub fn list<T: DeserializeOwned>(&self, collection: &str) -> Result<Vec<T>> {
        let tree = self.db.open_tree(collection)?;
        let mut items = Vec::new();

        for result in tree.iter() {
            let (_key, value) = result?;
            let item: T = serde_json::from_slice(&value)?;
            items.push(item);
        }

        Ok(items)
    }

    pub fn delete(&self, collection: &str, key: &str) -> Result<bool> {
        let tree = self.db.open_tree(collection)?;
        let existed = tree.remove(key)?.is_some();
        self.db.flush()?;
        if existed {
            if let Some(rep) = &self.replicator {
                let table = collection.to_string();
                let id = key.to_string();
                let rep = rep.clone();
                tokio::spawn(async move {
                    let _ = rep.delete(&table, &id).await;
                });
            }
        }
        Ok(existed)
    }

    // FIXED: Returns Result<()> instead of Result<bool>
    pub fn update<T: Serialize>(&self, collection: &str, key: &str, value: &T) -> Result<()> {
        self.insert(collection, key, value)
    }
    pub fn flush(&self) -> Result<usize> {
        Ok(self.db.flush()?)
    }
}

#[cfg(test)]
mod db_tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use tempfile::tempdir;

    #[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
    struct TestItem {
        id: String,
        name: String,
    }

    #[test]
    fn test_db_crud_operations() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::new(db_path.to_str().unwrap()).unwrap();

        let item = TestItem {
            id: "1".into(),
            name: "Test".into(),
        };

        // Insert
        db.insert("test_items", "1", &item).unwrap();

        // Get
        let retrieved: Option<TestItem> = db.get("test_items", "1").unwrap();
        assert_eq!(retrieved, Some(item.clone()));

        // List
        let items: Vec<TestItem> = db.list("test_items").unwrap();
        assert_eq!(items.len(), 1);

        // Update
        let updated = TestItem {
            id: "1".into(),
            name: "Updated".into(),
        };
        db.update("test_items", "1", &updated).unwrap();
        let retrieved: Option<TestItem> = db.get("test_items", "1").unwrap();
        assert_eq!(retrieved.unwrap().name, "Updated");

        // Delete
        let deleted = db.delete("test_items", "1").unwrap();
        assert!(deleted);
        let retrieved: Option<TestItem> = db.get("test_items", "1").unwrap();
        assert!(retrieved.is_none());
    }
}
