mod service;

use service::{
    Storage, UpdateGolink, create_golink, delete_golink, get_all_golinks, get_golink,
    update_golink, with_storage,
};
use warp::Filter;

#[tokio::main]
async fn main() {
    let storage: Storage = service::create_storage();

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
