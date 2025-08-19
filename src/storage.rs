use crate::service::Golink;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

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
    async fn get_paginated(&self, page: usize, page_size: usize) -> StorageResult<(Vec<Golink>, usize)>;
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

    async fn get_paginated(&self, page: usize, page_size: usize) -> StorageResult<(Vec<Golink>, usize)> {
        let store = self.data.read().await;
        let mut all_golinks: Vec<Golink> = store.values().cloned().collect();
        all_golinks.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        let total_items = all_golinks.len();
        let offset = (page.saturating_sub(1)) * page_size;
        
        let paginated_items = if offset < total_items {
            all_golinks.into_iter().skip(offset).take(page_size).collect()
        } else {
            Vec::new()
        };

        Ok((paginated_items, total_items))
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
        // Ensure the database URL has the proper format and create directories if needed
        let formatted_url = if database_url.starts_with("sqlite://") {
            database_url.to_string()
        } else {
            // Handle relative and absolute file paths
            let path = std::path::Path::new(database_url);
            
            // Create parent directories if they don't exist
            if let Some(parent) = path.parent() {
                if !parent.exists() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| sqlx::Error::Io(e))?;
                }
            }
            
            // Convert to proper SQLite URL format
            let absolute_path = path.canonicalize()
                .or_else(|_| {
                    // If canonicalize fails (file doesn't exist yet), use absolute path
                    if path.is_absolute() {
                        Ok(path.to_path_buf())
                    } else {
                        std::env::current_dir()
                            .map(|cwd| cwd.join(path))
                            .map_err(|e| sqlx::Error::Io(e))
                    }
                })?;
            
            format!("sqlite://{}", absolute_path.display())
        };

        // Use SqliteConnectOptions to enable database creation
        use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode};
        use std::str::FromStr;
        
        let connect_options = SqliteConnectOptions::from_str(&formatted_url)?
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal);
            
        let pool = sqlx::SqlitePool::connect_with(connect_options).await?;

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
            "INSERT INTO golinks (id, short_link, url, created_at) VALUES (?, ?, ?, ?)",
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
            "SELECT id, short_link, url, created_at FROM golinks WHERE short_link = ?",
        )
        .bind(short_link)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        row.ok_or(StorageError::NotFound)
    }

    async fn get_all(&self) -> StorageResult<Vec<Golink>> {
        let rows = sqlx::query_as::<_, Golink>(
            "SELECT id, short_link, url, created_at FROM golinks ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        Ok(rows)
    }

    async fn get_paginated(&self, page: usize, page_size: usize) -> StorageResult<(Vec<Golink>, usize)> {
        let offset = (page.saturating_sub(1)) * page_size;

        // Get total count
        let total_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM golinks")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        // Get paginated results
        let rows = sqlx::query_as::<_, Golink>(
            "SELECT id, short_link, url, created_at FROM golinks ORDER BY created_at DESC LIMIT ? OFFSET ?",
        )
        .bind(page_size as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        Ok((rows, total_count as usize))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::Golink;
    use tempfile::NamedTempFile;

    fn create_test_golink(short_link: &str, url: &str) -> Golink {
        Golink {
            id: uuid::Uuid::new_v4().to_string(),
            short_link: short_link.to_string(),
            url: url.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    mod hashmap_storage_tests {
        use super::*;

        #[tokio::test]
        async fn test_create_and_get_golink() {
            let storage = HashMapStorage::new();
            let golink = create_test_golink("go/test", "https://example.com");

            // Test create
            let result = storage.create(golink.clone()).await;
            assert!(result.is_ok());

            // Test get
            let retrieved = storage.get(&golink.short_link).await;
            assert!(retrieved.is_ok());
            let retrieved_golink = retrieved.unwrap();
            assert_eq!(retrieved_golink.short_link, golink.short_link);
            assert_eq!(retrieved_golink.url, golink.url);
        }

        #[tokio::test]
        async fn test_create_duplicate_returns_error() {
            let storage = HashMapStorage::new();
            let golink = create_test_golink("go/test", "https://example.com");

            // Create first time
            let result1 = storage.create(golink.clone()).await;
            assert!(result1.is_ok());

            // Create second time should fail
            let result2 = storage.create(golink.clone()).await;
            assert!(matches!(result2, Err(StorageError::AlreadyExists)));
        }

        #[tokio::test]
        async fn test_get_nonexistent_returns_not_found() {
            let storage = HashMapStorage::new();
            let result = storage.get("go/nonexistent").await;
            assert!(matches!(result, Err(StorageError::NotFound)));
        }

        #[tokio::test]
        async fn test_get_all_golinks() {
            let storage = HashMapStorage::new();
            let golink1 = create_test_golink("go/test1", "https://example1.com");
            let golink2 = create_test_golink("go/test2", "https://example2.com");

            storage.create(golink1.clone()).await.unwrap();
            storage.create(golink2.clone()).await.unwrap();

            let all_golinks = storage.get_all().await.unwrap();
            assert_eq!(all_golinks.len(), 2);
        }

        #[tokio::test]
        async fn test_update_golink() {
            let storage = HashMapStorage::new();
            let golink = create_test_golink("go/test", "https://example.com");

            storage.create(golink.clone()).await.unwrap();

            let updated = storage
                .update(&golink.short_link, "https://updated.com".to_string())
                .await;
            assert!(updated.is_ok());
            let updated_golink = updated.unwrap();
            assert_eq!(updated_golink.url, "https://updated.com");
        }

        #[tokio::test]
        async fn test_update_nonexistent_returns_not_found() {
            let storage = HashMapStorage::new();
            let result = storage
                .update("go/nonexistent", "https://example.com".to_string())
                .await;
            assert!(matches!(result, Err(StorageError::NotFound)));
        }

        #[tokio::test]
        async fn test_delete_golink() {
            let storage = HashMapStorage::new();
            let golink = create_test_golink("go/test", "https://example.com");

            storage.create(golink.clone()).await.unwrap();

            let result = storage.delete(&golink.short_link).await;
            assert!(result.is_ok());

            // Verify it's deleted
            let get_result = storage.get(&golink.short_link).await;
            assert!(matches!(get_result, Err(StorageError::NotFound)));
        }

        #[tokio::test]
        async fn test_delete_nonexistent_returns_not_found() {
            let storage = HashMapStorage::new();
            let result = storage.delete("go/nonexistent").await;
            assert!(matches!(result, Err(StorageError::NotFound)));
        }

        #[tokio::test]
        async fn test_exists() {
            let storage = HashMapStorage::new();
            let golink = create_test_golink("go/test", "https://example.com");

            // Should not exist initially
            let exists_before = storage.exists(&golink.short_link).await.unwrap();
            assert!(!exists_before);

            // Create and check exists
            storage.create(golink.clone()).await.unwrap();
            let exists_after = storage.exists(&golink.short_link).await.unwrap();
            assert!(exists_after);
        }
    }

    #[cfg(feature = "sqlite-tests")]
    mod sqlite_storage_tests {
        use super::*;

        async fn create_test_sqlite_storage() -> SqliteStorage {
            let temp_file = NamedTempFile::new().unwrap();
            let db_path = temp_file.path().to_str().unwrap();
            // Use file:// prefix for SQLite URLs in tests
            let db_url = format!("sqlite://{}?mode=rwc", db_path);
            SqliteStorage::new(&db_url).await.unwrap()
        }

        #[tokio::test]
        async fn test_create_and_get_golink() {
            let storage = create_test_sqlite_storage().await;
            let golink = create_test_golink("go/test", "https://example.com");

            // Test create
            let result = storage.create(golink.clone()).await;
            assert!(result.is_ok());

            // Test get
            let retrieved = storage.get(&golink.short_link).await;
            assert!(retrieved.is_ok());
            let retrieved_golink = retrieved.unwrap();
            assert_eq!(retrieved_golink.short_link, golink.short_link);
            assert_eq!(retrieved_golink.url, golink.url);
        }

        #[tokio::test]
        async fn test_create_duplicate_returns_error() {
            let storage = create_test_sqlite_storage().await;
            let golink = create_test_golink("go/test", "https://example.com");

            // Create first time
            let result1 = storage.create(golink.clone()).await;
            assert!(result1.is_ok());

            // Create second time should fail
            let result2 = storage.create(golink.clone()).await;
            assert!(matches!(result2, Err(StorageError::AlreadyExists)));
        }

        #[tokio::test]
        async fn test_get_nonexistent_returns_not_found() {
            let storage = create_test_sqlite_storage().await;
            let result = storage.get("go/nonexistent").await;
            assert!(matches!(result, Err(StorageError::NotFound)));
        }

        #[tokio::test]
        async fn test_get_all_golinks() {
            let storage = create_test_sqlite_storage().await;
            let golink1 = create_test_golink("go/test1", "https://example1.com");
            let golink2 = create_test_golink("go/test2", "https://example2.com");

            storage.create(golink1.clone()).await.unwrap();
            storage.create(golink2.clone()).await.unwrap();

            let all_golinks = storage.get_all().await.unwrap();
            assert_eq!(all_golinks.len(), 2);
        }

        #[tokio::test]
        async fn test_update_golink() {
            let storage = create_test_sqlite_storage().await;
            let golink = create_test_golink("go/test", "https://example.com");

            storage.create(golink.clone()).await.unwrap();

            let updated = storage
                .update(&golink.short_link, "https://updated.com".to_string())
                .await;
            assert!(updated.is_ok());
            let updated_golink = updated.unwrap();
            assert_eq!(updated_golink.url, "https://updated.com");
        }

        #[tokio::test]
        async fn test_update_nonexistent_returns_not_found() {
            let storage = create_test_sqlite_storage().await;
            let result = storage
                .update("go/nonexistent", "https://example.com".to_string())
                .await;
            assert!(matches!(result, Err(StorageError::NotFound)));
        }

        #[tokio::test]
        async fn test_delete_golink() {
            let storage = create_test_sqlite_storage().await;
            let golink = create_test_golink("go/test", "https://example.com");

            storage.create(golink.clone()).await.unwrap();

            let result = storage.delete(&golink.short_link).await;
            assert!(result.is_ok());

            // Verify it's deleted
            let get_result = storage.get(&golink.short_link).await;
            assert!(matches!(get_result, Err(StorageError::NotFound)));
        }

        #[tokio::test]
        async fn test_delete_nonexistent_returns_not_found() {
            let storage = create_test_sqlite_storage().await;
            let result = storage.delete("go/nonexistent").await;
            assert!(matches!(result, Err(StorageError::NotFound)));
        }

        #[tokio::test]
        async fn test_exists() {
            let storage = create_test_sqlite_storage().await;
            let golink = create_test_golink("go/test", "https://example.com");

            // Should not exist initially
            let exists_before = storage.exists(&golink.short_link).await.unwrap();
            assert!(!exists_before);

            // Create and check exists
            storage.create(golink.clone()).await.unwrap();
            let exists_after = storage.exists(&golink.short_link).await.unwrap();
            assert!(exists_after);
        }

        #[tokio::test]
        async fn test_persistence_across_connections() {
            let temp_file = NamedTempFile::new().unwrap();
            let db_path = temp_file.path().to_str().unwrap();
            let db_url = format!("sqlite://{}?mode=rwc", db_path);

            let golink = create_test_golink("go/persistent", "https://example.com");

            // Create storage, add golink, drop storage
            {
                let storage = SqliteStorage::new(&db_url).await.unwrap();
                storage.create(golink.clone()).await.unwrap();
            }

            // Create new storage instance, verify data persists
            {
                let storage = SqliteStorage::new(&db_url).await.unwrap();
                let retrieved = storage.get(&golink.short_link).await.unwrap();
                assert_eq!(retrieved.short_link, golink.short_link);
                assert_eq!(retrieved.url, golink.url);
            }
        }
    }
}
