use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::service::Golink;

#[derive(Debug)]
pub enum StorageError {
    NotFound,
    AlreadyExists,
    DatabaseError(String),
}

pub type StorageResult<T> = Result<T, StorageError>;

#[async_trait]
pub trait GoStorage: Send + Sync {
    async fn create(&self, golink: Golink) -> StorageResult<()>;
    async fn get(&self, short_link: &str) -> StorageResult<Golink>;
    async fn get_all(&self) -> StorageResult<Vec<Golink>>;
    async fn update(&self, short_link: &str, url: String) -> StorageResult<Golink>;
    async fn delete(&self, short_link: &str) -> StorageResult<()>;
    async fn exists(&self, short_link: &str) -> StorageResult<bool>;
}

// In-memory HashMap storage implementation
pub struct HashMapStorage {
    data: Arc<RwLock<HashMap<String, Golink>>>,
}

impl HashMapStorage {
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl GoStorage for HashMapStorage {
    async fn create(&self, golink: Golink) -> StorageResult<()> {
        let mut store = self.data.write().await;
        if store.contains_key(&golink.short_link) {
            return Err(StorageError::AlreadyExists);
        }
        store.insert(golink.short_link.clone(), golink);
        Ok(())
    }

    async fn get(&self, short_link: &str) -> StorageResult<Golink> {
        let store = self.data.read().await;
        store.get(short_link).cloned().ok_or(StorageError::NotFound)
    }

    async fn get_all(&self) -> StorageResult<Vec<Golink>> {
        let store = self.data.read().await;
        Ok(store.values().cloned().collect())
    }

    async fn update(&self, short_link: &str, url: String) -> StorageResult<Golink> {
        let mut store = self.data.write().await;
        match store.get_mut(short_link) {
            Some(golink) => {
                golink.url = url;
                Ok(golink.clone())
            }
            None => Err(StorageError::NotFound),
        }
    }

    async fn delete(&self, short_link: &str) -> StorageResult<()> {
        let mut store = self.data.write().await;
        store.remove(short_link).ok_or(StorageError::NotFound)?;
        Ok(())
    }

    async fn exists(&self, short_link: &str) -> StorageResult<bool> {
        let store = self.data.read().await;
        Ok(store.contains_key(short_link))
    }
}

// SQLite storage implementation
pub struct SqliteStorage {
    pool: sqlx::SqlitePool,
}

impl SqliteStorage {
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = sqlx::SqlitePool::connect(database_url).await?;
        
        // Create table if it doesn't exist
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS golinks (
                id TEXT PRIMARY KEY,
                short_link TEXT UNIQUE NOT NULL,
                url TEXT NOT NULL,
                created_at TEXT NOT NULL
            )
            "#,
        )
        .execute(&pool)
        .await?;

        Ok(Self { pool })
    }
}

#[async_trait]
impl GoStorage for SqliteStorage {
    async fn create(&self, golink: Golink) -> StorageResult<()> {
        let result = sqlx::query(
            "INSERT INTO golinks (id, short_link, url, created_at) VALUES (?, ?, ?, ?)"
        )
        .bind(&golink.id)
        .bind(&golink.short_link)
        .bind(&golink.url)
        .bind(&golink.created_at)
        .execute(&self.pool)
        .await;

        match result {
            Ok(_) => Ok(()),
            Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
                Err(StorageError::AlreadyExists)
            }
            Err(e) => Err(StorageError::DatabaseError(e.to_string())),
        }
    }

    async fn get(&self, short_link: &str) -> StorageResult<Golink> {
        let row = sqlx::query_as::<_, Golink>(
            "SELECT id, short_link, url, created_at FROM golinks WHERE short_link = ?"
        )
        .bind(short_link)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        row.ok_or(StorageError::NotFound)
    }

    async fn get_all(&self) -> StorageResult<Vec<Golink>> {
        let rows = sqlx::query_as::<_, Golink>(
            "SELECT id, short_link, url, created_at FROM golinks ORDER BY created_at DESC"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        Ok(rows)
    }

    async fn update(&self, short_link: &str, url: String) -> StorageResult<Golink> {
        let result = sqlx::query("UPDATE golinks SET url = ? WHERE short_link = ?")
            .bind(&url)
            .bind(short_link)
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(StorageError::NotFound);
        }

        // Fetch the updated record
        self.get(short_link).await
    }

    async fn delete(&self, short_link: &str) -> StorageResult<()> {
        let result = sqlx::query("DELETE FROM golinks WHERE short_link = ?")
            .bind(short_link)
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(StorageError::NotFound);
        }

        Ok(())
    }

    async fn exists(&self, short_link: &str) -> StorageResult<bool> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM golinks WHERE short_link = ?")
            .bind(short_link)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        Ok(count > 0)
    }
}