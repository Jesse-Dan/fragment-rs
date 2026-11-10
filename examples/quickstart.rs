use axum::Json;
use fragment_rs::{controller, route, service, FragmentApp, Inject, GET, POST};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Customer {
    id: String,
    name: String,
    email: String,
}

/// The customers a new repository starts with.
#[derive(Clone)]
struct SeededCustomers(Vec<Customer>);

impl Default for SeededCustomers {
    fn default() -> Self {
        SeededCustomers(vec![
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
        ])
    }
}

/// A repository that keeps its customers in memory.
///
/// The container hands out an `Arc`, so the data is shared through a lock.
///
/// Deriving `Default` is what lets the service register itself: the framework
/// builds it with `Default` and puts it in the container.
#[service]
#[derive(Default)]
struct CustomerRepository {
    customers: Arc<Mutex<SeededCustomers>>,
}

impl CustomerRepository {
    fn list(&self) -> Vec<Customer> {
        self.customers.lock().unwrap().0.clone()
    }

    fn create(&self, customer: Customer) {
        self.customers.lock().unwrap().0.push(customer);
    }
}

struct CustomerController;

#[controller("/api/customers")]
impl CustomerController {
    #[route(GET, "/")]
    async fn list(Inject(repo): Inject<CustomerRepository>) -> Json<Vec<Customer>> {
        Json(repo.list())
    }

    #[route(GET, "/:id")]
    async fn get_customer(
        axum::extract::Path(id): axum::extract::Path<String>,
        Inject(repo): Inject<CustomerRepository>,
    ) -> Result<Json<Customer>, axum::http::StatusCode> {
        repo.list()
            .into_iter()
            .find(|customer| customer.id == id)
            .map(Json)
            .ok_or(axum::http::StatusCode::NOT_FOUND)
    }

    #[route(POST, "/")]
    async fn create_customer(
        Inject(repo): Inject<CustomerRepository>,
        body: axum::Json<CreateCustomer>,
    ) -> (axum::http::StatusCode, Json<Customer>) {
        let new_customer = Customer {
            id: "3".into(), // In production, use the uuid crate
            name: body.name.clone(),
            email: body.email.clone(),
        };
        repo.create(new_customer.clone());
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
    // The controller and the service register themselves, so there is nothing
    // to wire up here.
    FragmentApp::new()
        .port(3001) // Avoid port conflict with tests
        .cors_origins(vec!["*".into()])
        .start()
        .await;
}
