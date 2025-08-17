use crate::storage::{GoStorage, StorageError};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;
use warp::Filter;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Golink {
    pub id: String,
    pub short_link: String,
    pub url: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateGolink {
    pub short_link: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateGolink {
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaginationInfo {
    pub page: usize,
    pub page_size: usize,
    pub total_items: usize,
    pub total_pages: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub pagination: PaginationInfo,
}

pub type Storage = Arc<dyn GoStorage>;

pub fn with_storage(
    storage: Storage,
) -> impl Filter<Extract = (Storage,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || storage.clone())
}

fn validate_golink_pattern(short_link: &str) -> Result<(), &'static str> {
    let re = Regex::new(r"^go/[a-zA-Z0-9_-]+$").unwrap();
    if re.is_match(short_link) {
        Ok(())
    } else {
        Err("Invalid golink pattern. Must match 'go/[a-zA-Z0-9_-]+'")
    }
}

pub async fn create_golink(
    create_golink: CreateGolink,
    storage: Storage,
) -> Result<warp::reply::WithStatus<warp::reply::Json>, warp::Rejection> {
    if let Err(e) = validate_golink_pattern(&create_golink.short_link) {
        let error_response = serde_json::json!({"error": e});
        return Ok(warp::reply::with_status(
            warp::reply::json(&error_response),
            warp::http::StatusCode::BAD_REQUEST,
        ));
    }

    let golink = Golink {
        id: Uuid::new_v4().to_string(),
        short_link: create_golink.short_link.clone(),
        url: create_golink.url,
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    match storage.create(golink.clone()).await {
        Ok(_) => Ok(warp::reply::with_status(
            warp::reply::json(&golink),
            warp::http::StatusCode::CREATED,
        )),
        Err(StorageError::AlreadyExists) => {
            let error_response = serde_json::json!({"error": "Golink already exists"});
            Ok(warp::reply::with_status(
                warp::reply::json(&error_response),
                warp::http::StatusCode::CONFLICT,
            ))
        }
        Err(StorageError::DatabaseError(e)) => {
            let error_response = serde_json::json!({"error": format!("Database error: {}", e)});
            Ok(warp::reply::with_status(
                warp::reply::json(&error_response),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
        Err(StorageError::NotFound) => {
            let error_response = serde_json::json!({"error": "Unexpected error"});
            Ok(warp::reply::with_status(
                warp::reply::json(&error_response),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

pub async fn get_golink(
    short_link: String,
    storage: Storage,
) -> Result<warp::reply::WithStatus<warp::reply::Json>, warp::Rejection> {
    match storage.get(&short_link).await {
        Ok(golink) => Ok(warp::reply::with_status(
            warp::reply::json(&golink),
            warp::http::StatusCode::OK,
        )),
        Err(StorageError::NotFound) => Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({"error": "Golink not found"})),
            warp::http::StatusCode::NOT_FOUND,
        )),
        Err(StorageError::DatabaseError(e)) => {
            let error_response = serde_json::json!({"error": format!("Database error: {}", e)});
            Ok(warp::reply::with_status(
                warp::reply::json(&error_response),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
        Err(_) => {
            let error_response = serde_json::json!({"error": "Unexpected error"});
            Ok(warp::reply::with_status(
                warp::reply::json(&error_response),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

pub async fn get_all_golinks(
    params: std::collections::HashMap<String, String>,
    storage: Storage,
) -> Result<warp::reply::WithStatus<warp::reply::Json>, warp::Rejection> {
    // Parse pagination parameters
    let page = params
        .get("page")
        .and_then(|p| p.parse::<usize>().ok())
        .unwrap_or(1)
        .max(1);
    
    let page_size = params
        .get("page_size")
        .and_then(|p| p.parse::<usize>().ok())
        .unwrap_or(10)
        .min(100)
        .max(1);

    // Check if pagination is requested
    let use_pagination = params.contains_key("page") || params.contains_key("page_size");

    if use_pagination {
        match storage.get_paginated(page, page_size).await {
            Ok((golinks, total_items)) => {
                let total_pages = (total_items + page_size - 1) / page_size;
                let pagination_info = PaginationInfo {
                    page,
                    page_size,
                    total_items,
                    total_pages,
                };
                let response = PaginatedResponse {
                    data: golinks,
                    pagination: pagination_info,
                };
                Ok(warp::reply::with_status(
                    warp::reply::json(&response),
                    warp::http::StatusCode::OK,
                ))
            }
            Err(StorageError::DatabaseError(e)) => {
                let error_response = serde_json::json!({"error": format!("Database error: {}", e)});
                Ok(warp::reply::with_status(
                    warp::reply::json(&error_response),
                    warp::http::StatusCode::INTERNAL_SERVER_ERROR,
                ))
            }
            Err(_) => {
                let error_response = serde_json::json!({"error": "Unexpected error"});
                Ok(warp::reply::with_status(
                    warp::reply::json(&error_response),
                    warp::http::StatusCode::INTERNAL_SERVER_ERROR,
                ))
            }
        }
    } else {
        // Return all items without pagination for backward compatibility
        match storage.get_all().await {
            Ok(golinks) => Ok(warp::reply::with_status(
                warp::reply::json(&golinks),
                warp::http::StatusCode::OK,
            )),
            Err(StorageError::DatabaseError(e)) => {
                let error_response = serde_json::json!({"error": format!("Database error: {}", e)});
                Ok(warp::reply::with_status(
                    warp::reply::json(&error_response),
                    warp::http::StatusCode::INTERNAL_SERVER_ERROR,
                ))
            }
            Err(_) => {
                let error_response = serde_json::json!({"error": "Unexpected error"});
                Ok(warp::reply::with_status(
                    warp::reply::json(&error_response),
                    warp::http::StatusCode::INTERNAL_SERVER_ERROR,
                ))
            }
        }
    }
}

pub async fn update_golink(
    short_link: String,
    update_golink: UpdateGolink,
    storage: Storage,
) -> Result<warp::reply::WithStatus<warp::reply::Json>, warp::Rejection> {
    match storage.update(&short_link, update_golink.url).await {
        Ok(golink) => Ok(warp::reply::with_status(
            warp::reply::json(&golink),
            warp::http::StatusCode::OK,
        )),
        Err(StorageError::NotFound) => Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({"error": "Golink not found"})),
            warp::http::StatusCode::NOT_FOUND,
        )),
        Err(StorageError::DatabaseError(e)) => {
            let error_response = serde_json::json!({"error": format!("Database error: {}", e)});
            Ok(warp::reply::with_status(
                warp::reply::json(&error_response),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
        Err(_) => {
            let error_response = serde_json::json!({"error": "Unexpected error"});
            Ok(warp::reply::with_status(
                warp::reply::json(&error_response),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

pub async fn delete_golink(
    short_link: String,
    storage: Storage,
) -> Result<warp::reply::WithStatus<warp::reply::Json>, warp::Rejection> {
    match storage.delete(&short_link).await {
        Ok(_) => Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({"message": "Golink deleted successfully"})),
            warp::http::StatusCode::OK,
        )),
        Err(StorageError::NotFound) => Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({"error": "Golink not found"})),
            warp::http::StatusCode::NOT_FOUND,
        )),
        Err(StorageError::DatabaseError(e)) => {
            let error_response = serde_json::json!({"error": format!("Database error: {}", e)});
            Ok(warp::reply::with_status(
                warp::reply::json(&error_response),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
        Err(_) => {
            let error_response = serde_json::json!({"error": "Unexpected error"});
            Ok(warp::reply::with_status(
                warp::reply::json(&error_response),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::HashMapStorage;
    use std::sync::Arc;
    use warp::Reply;

    async fn create_test_storage() -> Storage {
        Arc::new(HashMapStorage::new())
    }

    fn create_test_golink(short_link: &str, url: &str) -> Golink {
        Golink {
            id: uuid::Uuid::new_v4().to_string(),
            short_link: short_link.to_string(),
            url: url.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    #[test]
    fn test_validate_golink_pattern_valid() {
        assert!(validate_golink_pattern("go/test").is_ok());
        assert!(validate_golink_pattern("go/my-link").is_ok());
        assert!(validate_golink_pattern("go/my_link").is_ok());
        assert!(validate_golink_pattern("go/MyLink").is_ok());
        assert!(validate_golink_pattern("go/test123").is_ok());
        assert!(validate_golink_pattern("go/version2").is_ok());
        assert!(validate_golink_pattern("go/123test").is_ok());
    }

    #[test]
    fn test_validate_golink_pattern_invalid() {
        assert!(validate_golink_pattern("invalid").is_err());
        assert!(validate_golink_pattern("go/").is_err());
        assert!(validate_golink_pattern("go/test@").is_err());
        assert!(validate_golink_pattern("go/test space").is_err());
        assert!(validate_golink_pattern("notgo/test").is_err());
    }

    #[tokio::test]
    async fn test_create_golink_success() {
        let storage = create_test_storage().await;
        let create_req = CreateGolink {
            short_link: "go/test".to_string(),
            url: "https://example.com".to_string(),
        };

        let response = create_golink(create_req, storage).await;
        assert!(response.is_ok());
        
        let reply = response.unwrap();
        let status = reply.into_response().status();
        assert_eq!(status, warp::http::StatusCode::CREATED);
    }

    #[tokio::test]
    async fn test_create_golink_invalid_pattern() {
        let storage = create_test_storage().await;
        let create_req = CreateGolink {
            short_link: "invalid".to_string(),
            url: "https://example.com".to_string(),
        };

        let response = create_golink(create_req, storage).await;
        assert!(response.is_ok());
        
        let reply = response.unwrap();
        let status = reply.into_response().status();
        assert_eq!(status, warp::http::StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_create_golink_already_exists() {
        let storage = create_test_storage().await;
        let golink = create_test_golink("go/test", "https://example.com");

        // Pre-populate storage
        storage.create(golink.clone()).await.unwrap();

        let create_req = CreateGolink {
            short_link: "go/test".to_string(),
            url: "https://example.com".to_string(),
        };

        let response = create_golink(create_req, storage).await;
        assert!(response.is_ok());
        
        let reply = response.unwrap();
        let status = reply.into_response().status();
        assert_eq!(status, warp::http::StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn test_get_golink_success() {
        let storage = create_test_storage().await;
        let golink = create_test_golink("go/test", "https://example.com");

        // Pre-populate storage
        storage.create(golink.clone()).await.unwrap();

        let response = get_golink("go/test".to_string(), storage).await;
        assert!(response.is_ok());
        
        let reply = response.unwrap();
        let status = reply.into_response().status();
        assert_eq!(status, warp::http::StatusCode::OK);
    }

    #[tokio::test]
    async fn test_get_golink_not_found() {
        let storage = create_test_storage().await;

        let response = get_golink("go/nonexistent".to_string(), storage).await;
        assert!(response.is_ok());

        let reply = response.unwrap();
        let status = reply.into_response().status();
        assert_eq!(status, warp::http::StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_get_all_golinks() {
        let storage = create_test_storage().await;
        let golink1 = create_test_golink("go/test1", "https://example1.com");
        let golink2 = create_test_golink("go/test2", "https://example2.com");

        // Pre-populate storage
        storage.create(golink1).await.unwrap();
        storage.create(golink2).await.unwrap();

        let params = std::collections::HashMap::new();
        let response = get_all_golinks(params, storage).await;
        assert!(response.is_ok());
        
        let reply = response.unwrap();
        let status = reply.into_response().status();
        assert_eq!(status, warp::http::StatusCode::OK);
    }

    #[tokio::test]
    async fn test_get_all_golinks_with_pagination() {
        let storage = create_test_storage().await;
        let golink1 = create_test_golink("go/test1", "https://example1.com");
        let golink2 = create_test_golink("go/test2", "https://example2.com");
        let golink3 = create_test_golink("go/test3", "https://example3.com");

        // Pre-populate storage
        storage.create(golink1).await.unwrap();
        storage.create(golink2).await.unwrap();
        storage.create(golink3).await.unwrap();

        let mut params = std::collections::HashMap::new();
        params.insert("page".to_string(), "1".to_string());
        params.insert("page_size".to_string(), "2".to_string());

        let response = get_all_golinks(params, storage).await;
        assert!(response.is_ok());
        
        let reply = response.unwrap();
        let status = reply.into_response().status();
        assert_eq!(status, warp::http::StatusCode::OK);
    }

    #[tokio::test]
    async fn test_update_golink_success() {
        let storage = create_test_storage().await;
        let golink = create_test_golink("go/test", "https://example.com");

        // Pre-populate storage
        storage.create(golink.clone()).await.unwrap();

        let update_req = UpdateGolink {
            url: "https://updated.com".to_string(),
        };

        let response = update_golink("go/test".to_string(), update_req, storage).await;
        assert!(response.is_ok());
        
        let reply = response.unwrap();
        let status = reply.into_response().status();
        assert_eq!(status, warp::http::StatusCode::OK);
    }

    #[tokio::test]
    async fn test_update_golink_not_found() {
        let storage = create_test_storage().await;

        let update_req = UpdateGolink {
            url: "https://updated.com".to_string(),
        };

        let response = update_golink("go/nonexistent".to_string(), update_req, storage).await;
        assert!(response.is_ok());
        
        let reply = response.unwrap();
        let status = reply.into_response().status();
        assert_eq!(status, warp::http::StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_delete_golink_success() {
        let storage = create_test_storage().await;
        let golink = create_test_golink("go/test", "https://example.com");

        // Pre-populate storage
        storage.create(golink.clone()).await.unwrap();

        let response = delete_golink("go/test".to_string(), storage).await;
        assert!(response.is_ok());
        
        let reply = response.unwrap();
        let status = reply.into_response().status();
        assert_eq!(status, warp::http::StatusCode::OK);
    }

    #[tokio::test]
    async fn test_delete_golink_not_found() {
        let storage = create_test_storage().await;

        let response = delete_golink("go/nonexistent".to_string(), storage).await;
        assert!(response.is_ok());
        
        let reply = response.unwrap();
        let status = reply.into_response().status();
        assert_eq!(status, warp::http::StatusCode::NOT_FOUND);
    }
}
