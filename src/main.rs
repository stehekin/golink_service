mod service;
mod storage;

use service::{
    Storage, UpdateGolink, create_golink, delete_golink, get_all_golinks, get_golink,
    update_golink, with_storage,
};
use std::sync::Arc;
use storage::{HashMapStorage, SqliteStorage};
use warp::Filter;

#[tokio::main]
async fn main() {
    // Choose storage backend based on environment variable or default to in-memory
    let storage: Storage = if std::env::var("USE_SQLITE").is_ok() {
        let database_url =
            std::env::var("DATABASE_URL").unwrap_or_else(|_| "golinks.db".to_string());
        match SqliteStorage::new(&database_url).await {
            Ok(sqlite_storage) => Arc::new(sqlite_storage),
            Err(e) => {
                eprintln!("Failed to initialize SQLite storage: {}", e);
                eprintln!("Falling back to in-memory storage");
                Arc::new(HashMapStorage::new())
            }
        }
    } else {
        Arc::new(HashMapStorage::new())
    };

    let storage_type = if std::env::var("USE_SQLITE").is_ok() {
        "SQLite"
    } else {
        "In-memory HashMap"
    };
    println!("Using {} storage", storage_type);

    let create_route = warp::path("golinks")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_storage(storage.clone()))
        .and_then(create_golink);

    let get_all_route = warp::path("golinks")
        .and(warp::get())
        .and(with_storage(storage.clone()))
        .and_then(get_all_golinks);

    let get_route = warp::path("golinks")
        .and(warp::path::param::<String>())
        .and(warp::path::param::<String>())
        .and(warp::path::end())
        .and(warp::get())
        .and(with_storage(storage.clone()))
        .and_then(|prefix: String, name: String, storage: Storage| {
            get_golink(format!("{}/{}", prefix, name), storage)
        });

    let update_route = warp::path("golinks")
        .and(warp::path::param::<String>())
        .and(warp::path::param::<String>())
        .and(warp::path::end())
        .and(warp::put())
        .and(warp::body::json())
        .and(with_storage(storage.clone()))
        .and_then(
            |prefix: String, name: String, update_data: UpdateGolink, storage: Storage| {
                update_golink(format!("{}/{}", prefix, name), update_data, storage)
            },
        );

    let delete_route = warp::path("golinks")
        .and(warp::path::param::<String>())
        .and(warp::path::param::<String>())
        .and(warp::path::end())
        .and(warp::delete())
        .and(with_storage(storage.clone()))
        .and_then(|prefix: String, name: String, storage: Storage| {
            delete_golink(format!("{}/{}", prefix, name), storage)
        });

    let routes = create_route
        .or(get_all_route)
        .or(get_route)
        .or(update_route)
        .or(delete_route)
        .with(warp::cors().allow_any_origin());

    println!("Golink service running on http://localhost:3030");
    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}
