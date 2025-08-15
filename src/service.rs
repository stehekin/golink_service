use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use warp::Filter;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Golink {
    pub id: String,
    pub short_link: String,
    pub url: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateGolink {
    pub short_link: String,
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateGolink {
    pub url: String,
}

pub type Storage = Arc<RwLock<HashMap<String, Golink>>>;

pub fn create_storage() -> Storage {
    Arc::new(RwLock::new(HashMap::new()))
}

pub fn with_storage(
    storage: Storage,
) -> impl Filter<Extract = (Storage,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || storage.clone())
}

fn validate_golink_pattern(short_link: &str) -> Result<(), &'static str> {
    let re = Regex::new(r"^go/[a-zA-Z_-]+$").unwrap();
    if re.is_match(short_link) {
        Ok(())
    } else {
        Err("Invalid golink pattern. Must match 'go/[a-zA-Z_-]+'")
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

    let mut store = storage.write().await;

    if store.contains_key(&create_golink.short_link) {
        let error_response = serde_json::json!({"error": "Golink already exists"});
        return Ok(warp::reply::with_status(
            warp::reply::json(&error_response),
            warp::http::StatusCode::CONFLICT,
        ));
    }

    let golink = Golink {
        id: Uuid::new_v4().to_string(),
        short_link: create_golink.short_link.clone(),
        url: create_golink.url,
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    store.insert(create_golink.short_link, golink.clone());

    Ok(warp::reply::with_status(
        warp::reply::json(&golink),
        warp::http::StatusCode::CREATED,
    ))
}

pub async fn get_golink(
    short_link: String,
    storage: Storage,
) -> Result<warp::reply::WithStatus<warp::reply::Json>, warp::Rejection> {
    let store = storage.read().await;

    match store.get(&short_link) {
        Some(golink) => Ok(warp::reply::with_status(
            warp::reply::json(golink),
            warp::http::StatusCode::OK,
        )),
        None => Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({"error": "Golink not found"})),
            warp::http::StatusCode::NOT_FOUND,
        )),
    }
}

pub async fn get_all_golinks(
    storage: Storage,
) -> Result<warp::reply::WithStatus<warp::reply::Json>, warp::Rejection> {
    let store = storage.read().await;
    let golinks: Vec<&Golink> = store.values().collect();

    Ok(warp::reply::with_status(
        warp::reply::json(&golinks),
        warp::http::StatusCode::OK,
    ))
}

pub async fn update_golink(
    short_link: String,
    update_golink: UpdateGolink,
    storage: Storage,
) -> Result<warp::reply::WithStatus<warp::reply::Json>, warp::Rejection> {
    let mut store = storage.write().await;

    match store.get_mut(&short_link) {
        Some(golink) => {
            golink.url = update_golink.url;
            Ok(warp::reply::with_status(
                warp::reply::json(golink),
                warp::http::StatusCode::OK,
            ))
        }
        None => Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({"error": "Golink not found"})),
            warp::http::StatusCode::NOT_FOUND,
        )),
    }
}

pub async fn delete_golink(
    short_link: String,
    storage: Storage,
) -> Result<warp::reply::WithStatus<warp::reply::Json>, warp::Rejection> {
    let mut store = storage.write().await;

    match store.remove(&short_link) {
        Some(_) => Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({"message": "Golink deleted successfully"})),
            warp::http::StatusCode::OK,
        )),
        None => Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({"error": "Golink not found"})),
            warp::http::StatusCode::NOT_FOUND,
        )),
    }
}
