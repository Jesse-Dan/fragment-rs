//! End-to-end checks of the request flow: routing, dependency injection,
//! validation, and error handling.

use axum::http::StatusCode;
use axum::Json;
use fragment_rs::{
    controller, core::container::Inject, core::validation::ValidatedJson, route, service, DELETE,
    GET, POST,
};
use serde::{Deserialize, Serialize};
use tower::ServiceExt;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Customer {
    id: String,
    name: String,
}

#[derive(Debug, Deserialize, Validate)]
struct CreateCustomer {
    #[validate(length(min = 2, message = "Name must be at least 2 characters"))]
    name: String,
    #[validate(email(message = "Invalid email address"))]
    email: String,
}

#[service]
#[derive(Default)]
struct CustomerRepository {
    customers: std::sync::Arc<std::sync::Mutex<Vec<Customer>>>,
}

struct CustomerController;

#[controller("/api/customers")]
impl CustomerController {
    #[route(GET, "/")]
    async fn list(Inject(repo): Inject<CustomerRepository>) -> Json<Vec<Customer>> {
        Json(repo.customers.lock().unwrap().clone())
    }

    #[route(POST, "/")]
    async fn create(
        Inject(repo): Inject<CustomerRepository>,
        ValidatedJson(body): ValidatedJson<CreateCustomer>,
    ) -> (StatusCode, Json<Customer>) {
        let customer = Customer {
            id: "1".to_owned(),
            name: body.name,
        };
        repo.customers.lock().unwrap().push(customer.clone());
        (StatusCode::CREATED, Json(customer))
    }
}

struct HealthController;

#[controller("/health")]
impl HealthController {
    #[route(GET, "/")]
    async fn health() -> Json<serde_json::Value> {
        Json(serde_json::json!({ "status": "ok" }))
    }
}

/// Builds the router the same way an application does.
///
/// Nothing is registered by hand: the controllers and the service find
/// themselves, which is what the `#[controller]` and `#[service]` macros are
/// for.
fn router() -> axum::Router {
    use fragment_rs::core::container::DIContainer;

    // The application does this when it starts.
    DIContainer::auto_register_services();

    let mut router = axum::Router::new();
    for register in fragment_rs::core::routing::registered_controllers() {
        router = register(router);
    }
    router
}

/// Sends a request and returns the status and body.
async fn call(
    router: axum::Router,
    method: &str,
    uri: &str,
    body: Option<&str>,
) -> (StatusCode, String) {
    let mut request = http::Request::new(axum::body::Body::empty());
    *request.method_mut() = http::Method::from_bytes(method.as_bytes()).unwrap();
    *request.uri_mut() = uri.parse::<http::Uri>().unwrap();
    if let Some(body) = body {
        request.headers_mut().insert(
            http::header::CONTENT_TYPE,
            http::HeaderValue::from_static("application/json"),
        );
        *request.body_mut() = axum::body::Body::from(body.to_owned());
    }

    let response = router.oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 4096)
        .await
        .unwrap();
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

#[tokio::test]
async fn a_registered_route_answers_without_a_trailing_slash() {
    let (status, body) = call(router(), "GET", "/api/customers", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "[]");
}

#[tokio::test]
async fn the_prefix_is_applied_to_every_route() {
    let (status, body) = call(router(), "GET", "/health", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("ok"));
}

#[tokio::test]
async fn a_valid_body_is_accepted() {
    let (status, body) = call(
        router(),
        "POST",
        "/api/customers",
        Some(r#"{"name":"Ada","email":"ada@example.com"}"#),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert!(body.contains("Ada"));
}

#[tokio::test]
async fn an_invalid_body_is_rejected_with_a_validation_error() {
    let (status, body) = call(
        router(),
        "POST",
        "/api/customers",
        Some(r#"{"name":"A","email":"not-an-email"}"#),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(body.contains("Validation failed"));
}

#[tokio::test]
async fn a_malformed_body_is_rejected_with_a_readable_error() {
    let (status, body) = call(router(), "POST", "/api/customers", Some("not json")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.contains("could not be parsed as JSON"));
}

/// A service that cannot build itself, so it is never registered.
///
/// A `#[service]` type that derives `Default` registers itself; one that does
/// not has to be registered by hand.
#[service]
struct UnregisteredService {
    value: u32,
}

struct NeedsUnregisteredController;

#[controller("/needs-unregistered")]
impl NeedsUnregisteredController {
    #[route(GET, "/")]
    async fn read(Inject(service): Inject<UnregisteredService>) -> Json<&'static str> {
        let _ = service.value;
        Json("unreachable")
    }
}

#[tokio::test]
async fn a_missing_service_does_not_leak_internals() {
    use fragment_rs::core::routing::Controller;

    let router = <NeedsUnregisteredController as Controller>::register_routes(axum::Router::new());
    let (status, body) = call(router, "GET", "/needs-unregistered", None).await;

    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    // The client is told that something went wrong, not which type was
    // missing or where it was looked up.
    assert!(body.contains("An unexpected error occurred"));
    assert!(!body.contains("UnregisteredService"));
    assert!(!body.contains("not registered"));
}

/// The method constants have to be in scope for the controller macro.
#[allow(dead_code)]
fn method_constants_are_exported() {
    let _ = (GET, POST, DELETE);
}
