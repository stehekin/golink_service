mod service;
mod storage;

use service::{
    Storage, UpdateGolink, create_golink, delete_golink, get_all_golinks, get_golink,
    update_golink, with_storage, with_auth, handle_auth_rejection,
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
                eprintln!("Error: Failed to initialize SQLite storage: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        Arc::new(HashMapStorage::new())
    };
    
    // Log authentication status
    if std::env::var("AUTH_TOKEN").is_ok() {
        println!("Authentication: ENABLED");
    } else {
        println!("Authentication: DISABLED");
    }

    let create_route = warp::path("golinks")
        .and(warp::post())
        .and(with_auth()) // Require authentication for creating golinks
        .and(warp::body::json())
        .and(with_storage(storage.clone()))
        .and_then(create_golink);

    let get_all_route = warp::path("golinks")
        .and(warp::path::end())
        .and(warp::get())
        .and(with_auth()) // Require authentication for getting all golinks
        .and(warp::query::<std::collections::HashMap<String, String>>())
        .and(with_storage(storage.clone()))
        .and_then(get_all_golinks);

    let get_route = warp::path("golinks")
        .and(warp::path::param::<String>())
        .and(warp::path::param::<String>())
        .and(warp::path::end())
        .and(warp::get())
        .and(with_auth()) // Require authentication for getting specific golinks
        .and(with_storage(storage.clone()))
        .and_then(|prefix: String, name: String, storage: Storage| {
            get_golink(format!("{}/{}", prefix, name), storage)
        });

    let update_route = warp::path("golinks")
        .and(warp::path::param::<String>())
        .and(warp::path::param::<String>())
        .and(warp::path::end())
        .and(warp::put())
        .and(with_auth()) // Require authentication for updating golinks
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
        .and(with_auth()) // Require authentication for deleting golinks
        .and(with_storage(storage.clone()))
        .and_then(|prefix: String, name: String, storage: Storage| {
            delete_golink(format!("{}/{}", prefix, name), storage)
        });

    // IMPORTANT: Route order matters! Specific routes must come before general routes.
    // get_route (/golinks/{prefix}/{name}) must come before get_all_route (/golinks)
    // to prevent the general route from matching specific golink requests.
    let routes = create_route
        .or(get_route)        // Specific: /golinks/{prefix}/{name}
        .or(update_route)     // Specific: /golinks/{prefix}/{name}
        .or(delete_route)     // Specific: /golinks/{prefix}/{name}
        .or(get_all_route)    // General: /golinks (must be last)
        .with(warp::cors().allow_any_origin())
        .recover(handle_auth_rejection);

    println!("Golink service running on http://localhost:3030");
    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}
