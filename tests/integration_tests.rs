use golink::service::{CreateGolink, UpdateGolink};
use golink::storage::HashMapStorage;
use std::sync::Arc;
use warp::test::request;
use warp::Filter;

// Helper function to create routes with in-memory storage
fn create_app() -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let storage = Arc::new(HashMapStorage::new());

    let create_route = warp::path("golinks")
        .and(warp::post())
        .and(warp::body::json())
        .and(golink::service::with_storage(storage.clone()))
        .and_then(golink::service::create_golink);

    let get_all_route = warp::path("golinks")
        .and(warp::path::end())
        .and(warp::get())
        .and(golink::service::with_storage(storage.clone()))
        .and_then(golink::service::get_all_golinks);

    let get_route = warp::path("golinks")
        .and(warp::path::param::<String>())
        .and(warp::path::param::<String>())
        .and(warp::path::end())
        .and(warp::get())
        .and(golink::service::with_storage(storage.clone()))
        .and_then(|prefix: String, name: String, storage| {
            golink::service::get_golink(format!("{}/{}", prefix, name), storage)
        });

    let update_route = warp::path("golinks")
        .and(warp::path::param::<String>())
        .and(warp::path::param::<String>())
        .and(warp::path::end())
        .and(warp::put())
        .and(warp::body::json())
        .and(golink::service::with_storage(storage.clone()))
        .and_then(|prefix: String, name: String, update_data: UpdateGolink, storage| {
            golink::service::update_golink(format!("{}/{}", prefix, name), update_data, storage)
        });

    let delete_route = warp::path("golinks")
        .and(warp::path::param::<String>())
        .and(warp::path::param::<String>())
        .and(warp::path::end())
        .and(warp::delete())
        .and(golink::service::with_storage(storage.clone()))
        .and_then(|prefix: String, name: String, storage| {
            golink::service::delete_golink(format!("{}/{}", prefix, name), storage)
        });

    create_route
        .or(get_route)
        .or(update_route)
        .or(delete_route)
        .or(get_all_route)
        .with(warp::cors().allow_any_origin())
}

#[tokio::test]
async fn test_create_golink_api() {
    let app = create_app();

    let create_req = CreateGolink {
        short_link: "go/test".to_string(),
        url: "https://example.com".to_string(),
    };

    let resp = request()
        .method("POST")
        .path("/golinks")
        .header("content-type", "application/json")
        .json(&create_req)
        .reply(&app)
        .await;

    assert_eq!(resp.status(), 201);
    
    let body: serde_json::Value = serde_json::from_slice(resp.body()).unwrap();
    assert_eq!(body["short_link"], "go/test");
    assert_eq!(body["url"], "https://example.com");
    assert!(body["id"].is_string());
    assert!(body["created_at"].is_string());
}

#[tokio::test]
async fn test_create_invalid_golink_pattern() {
    let app = create_app();

    let create_req = CreateGolink {
        short_link: "invalid".to_string(),
        url: "https://example.com".to_string(),
    };

    let resp = request()
        .method("POST")
        .path("/golinks")
        .header("content-type", "application/json")
        .json(&create_req)
        .reply(&app)
        .await;

    assert_eq!(resp.status(), 400);
    
    let body: serde_json::Value = serde_json::from_slice(resp.body()).unwrap();
    assert!(body["error"].as_str().unwrap().contains("Invalid golink pattern"));
}

#[tokio::test]
async fn test_get_all_golinks_empty() {
    let app = create_app();

    let resp = request()
        .method("GET")
        .path("/golinks")
        .reply(&app)
        .await;

    assert_eq!(resp.status(), 200);
    
    let body: serde_json::Value = serde_json::from_slice(resp.body()).unwrap();
    assert!(body.is_array());
    assert_eq!(body.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_create_and_get_golink() {
    let app = create_app();

    // Create a golink
    let create_req = CreateGolink {
        short_link: "go/test".to_string(),
        url: "https://example.com".to_string(),
    };

    let create_resp = request()
        .method("POST")
        .path("/golinks")
        .header("content-type", "application/json")
        .json(&create_req)
        .reply(&app)
        .await;

    assert_eq!(create_resp.status(), 201);

    // Get the golink
    let get_resp = request()
        .method("GET")
        .path("/golinks/go/test")
        .reply(&app)
        .await;

    assert_eq!(get_resp.status(), 200);
    
    let body: serde_json::Value = serde_json::from_slice(get_resp.body()).unwrap();
    assert!(body.is_object()); // Now returns individual golink object
    assert_eq!(body["short_link"], "go/test");
    assert_eq!(body["url"], "https://example.com");
}

#[tokio::test]
async fn test_get_nonexistent_golink() {
    let app = create_app();

    // First create a random golink to ensure storage is not empty
    let create_req = CreateGolink {
        short_link: "go/random".to_string(),
        url: "https://random.com".to_string(),
    };

    let create_resp = request()
        .method("POST")
        .path("/golinks")
        .header("content-type", "application/json")
        .json(&create_req)
        .reply(&app)
        .await;

    assert_eq!(create_resp.status(), 201);

    // Now test getting a nonexistent golink
    let resp = request()
        .method("GET")
        .path("/golinks/go/nonexistent")
        .reply(&app)
        .await;

    assert_eq!(resp.status(), 404); // Should return 404 for nonexistent golink
    
    let body: serde_json::Value = serde_json::from_slice(resp.body()).unwrap();
    assert!(body["error"].as_str().unwrap().contains("not found"));
}

#[tokio::test]
async fn test_create_duplicate_golink() {
    let app = create_app();

    let create_req = CreateGolink {
        short_link: "go/test".to_string(),
        url: "https://example.com".to_string(),
    };

    // Create first time
    let resp1 = request()
        .method("POST")
        .path("/golinks")
        .header("content-type", "application/json")
        .json(&create_req)
        .reply(&app)
        .await;

    assert_eq!(resp1.status(), 201);

    // Create second time should fail
    let resp2 = request()
        .method("POST")
        .path("/golinks")
        .header("content-type", "application/json")
        .json(&create_req)
        .reply(&app)
        .await;

    assert_eq!(resp2.status(), 409);
    
    let body: serde_json::Value = serde_json::from_slice(resp2.body()).unwrap();
    assert!(body["error"].as_str().unwrap().contains("already exists"));
}

#[tokio::test]
async fn test_update_golink() {
    let app = create_app();

    // Create a golink first
    let create_req = CreateGolink {
        short_link: "go/test".to_string(),
        url: "https://example.com".to_string(),
    };

    let create_resp = request()
        .method("POST")
        .path("/golinks")
        .header("content-type", "application/json")
        .json(&create_req)
        .reply(&app)
        .await;

    assert_eq!(create_resp.status(), 201);

    // Update the golink
    let update_req = UpdateGolink {
        url: "https://updated.com".to_string(),
    };

    let update_resp = request()
        .method("PUT")
        .path("/golinks/go/test")
        .header("content-type", "application/json")
        .json(&update_req)
        .reply(&app)
        .await;

    assert_eq!(update_resp.status(), 200);
    
    let body: serde_json::Value = serde_json::from_slice(update_resp.body()).unwrap();
    assert_eq!(body["url"], "https://updated.com");
}

#[tokio::test]
async fn test_update_nonexistent_golink() {
    let app = create_app();

    let update_req = UpdateGolink {
        url: "https://updated.com".to_string(),
    };

    let resp = request()
        .method("PUT")
        .path("/golinks/go/nonexistent")
        .header("content-type", "application/json")
        .json(&update_req)
        .reply(&app)
        .await;

    assert_eq!(resp.status(), 404);
    
    let body: serde_json::Value = serde_json::from_slice(resp.body()).unwrap();
    assert!(body["error"].as_str().unwrap().contains("not found"));
}

#[tokio::test]
async fn test_delete_golink() {
    let app = create_app();

    // Create a golink first
    let create_req = CreateGolink {
        short_link: "go/test".to_string(),
        url: "https://example.com".to_string(),
    };

    let create_resp = request()
        .method("POST")
        .path("/golinks")
        .header("content-type", "application/json")
        .json(&create_req)
        .reply(&app)
        .await;

    assert_eq!(create_resp.status(), 201);

    // Delete the golink
    let delete_resp = request()
        .method("DELETE")
        .path("/golinks/go/test")
        .reply(&app)
        .await;

    assert_eq!(delete_resp.status(), 200);
    
    let body: serde_json::Value = serde_json::from_slice(delete_resp.body()).unwrap();
    assert!(body["message"].as_str().unwrap().contains("deleted successfully"));
}

#[tokio::test]
async fn test_delete_nonexistent_golink() {
    let app = create_app();

    let resp = request()
        .method("DELETE")
        .path("/golinks/go/nonexistent")
        .reply(&app)
        .await;

    assert_eq!(resp.status(), 404);
    
    let body: serde_json::Value = serde_json::from_slice(resp.body()).unwrap();
    assert!(body["error"].as_str().unwrap().contains("not found"));
}

#[tokio::test]
async fn test_full_crud_workflow() {
    let app = create_app();

    // 1. Start with empty list
    let list_resp = request()
        .method("GET")
        .path("/golinks")
        .reply(&app)
        .await;
    assert_eq!(list_resp.status(), 200);
    let body: serde_json::Value = serde_json::from_slice(list_resp.body()).unwrap();
    assert_eq!(body.as_array().unwrap().len(), 0);

    // 2. Create a golink
    let create_req = CreateGolink {
        short_link: "go/example".to_string(),
        url: "https://example.com".to_string(),
    };

    let create_resp = request()
        .method("POST")
        .path("/golinks")
        .header("content-type", "application/json")
        .json(&create_req)
        .reply(&app)
        .await;
    assert_eq!(create_resp.status(), 201);

    // 3. Verify it appears in list
    let list_resp = request()
        .method("GET")
        .path("/golinks")
        .reply(&app)
        .await;
    assert_eq!(list_resp.status(), 200);
    let body: serde_json::Value = serde_json::from_slice(list_resp.body()).unwrap();
    assert_eq!(body.as_array().unwrap().len(), 1);

    // 4. Update the golink
    let update_req = UpdateGolink {
        url: "https://updated-example.com".to_string(),
    };

    let update_resp = request()
        .method("PUT")
        .path("/golinks/go/example")
        .header("content-type", "application/json")
        .json(&update_req)
        .reply(&app)
        .await;
    assert_eq!(update_resp.status(), 200);

    // 5. Delete the golink
    let delete_resp = request()
        .method("DELETE")
        .path("/golinks/go/example")
        .reply(&app)
        .await;
    assert_eq!(delete_resp.status(), 200);

    // 6. Verify it's gone from list
    let list_resp = request()
        .method("GET")
        .path("/golinks")
        .reply(&app)
        .await;
    assert_eq!(list_resp.status(), 200);
    let body: serde_json::Value = serde_json::from_slice(list_resp.body()).unwrap();
    assert_eq!(body.as_array().unwrap().len(), 0);
}