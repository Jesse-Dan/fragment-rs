use axum::Json;
use fragment_rs::{controller, route, service, FragmentApp, Inject};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Customer {
    id: String,
    name: String,
    email: String,
}

// Simple in-memory repository - note: uses a simple wrapper to avoid macro expansion issues with complex types
#[derive(Clone)]
struct InMemoryCustomerRepo(Vec<Customer>);

impl InMemoryCustomerRepo {
    fn new() -> Self {
        let initial = vec![
            Customer {
                id: "1".into(),
                name: "Alice".into(),
                email: "alice@example.com".into(),
            },
            Customer {
                id: "2".into(),
                name: "Bob".into(),
                email: "bob@example.com".into(),
            },
        ];
        Self(initial)
    }

    fn list(&self) -> Vec<Customer> {
        self.0.clone()
    }
}

#[service]
struct CustomerRepoWrapper {
    repo: InMemoryCustomerRepo,
}

impl Default for CustomerRepoWrapper {
    fn default() -> Self {
        Self {
            repo: InMemoryCustomerRepo::new(),
        }
    }
}

struct CustomerController;

#[controller("/api/customers")]
impl CustomerController {
    #[route(GET, "/")]
    async fn list(Inject(repo): Inject<CustomerRepoWrapper>) -> Json<Vec<Customer>> {
        Json(repo.repo.list())
    }

    #[route(GET, "/{id}")]
    async fn get_customer(
        axum::extract::Path(id): axum::extract::Path<String>,
        Inject(repo): Inject<CustomerRepoWrapper>,
    ) -> Result<Json<Customer>, axum::http::StatusCode> {
        let customers = repo.repo.list();
        customers
            .iter()
            .find(|c| c.id == id)
            .cloned()
            .map(Json)
            .ok_or(axum::http::StatusCode::NOT_FOUND)
    }

    #[route(POST, "/")]
    async fn create_customer(
        Inject(repo): Inject<CustomerRepoWrapper>,
        body: axum::Json<CreateCustomer>,
    ) -> (axum::http::StatusCode, Json<Customer>) {
        let new_customer = Customer {
            id: "3".into(), // In production, use uuid crate
            name: body.name.clone(),
            email: body.email.clone(),
        };
        (axum::http::StatusCode::CREATED, Json(new_customer))
    }
}

#[derive(Debug, Deserialize)]
struct CreateCustomer {
    name: String,
    email: String,
}

#[tokio::main]
async fn main() {
    FragmentApp::new()
        .port(3001) // Avoid port conflict with tests
        .cors_origins(vec!["*".into()])
        .start()
        .await;
}
