use axum::http::StatusCode;
use axum::Json;
use fragment_rs::{
    controller, core::container::Inject, core::validation::ValidatedJson, route, service,
    FragmentApp, GET, POST,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Customer {
    id: String,
    name: String,
    email: String,
}

#[derive(Debug, Deserialize, Validate)]
struct CreateCustomerDto {
    #[validate(length(min = 2))]
    name: String,
    #[validate(email)]
    email: String,
}

#[service]
struct CustomerRepository {
    customers: Arc<Mutex<Vec<Customer>>>,
}

impl CustomerRepository {
    fn new() -> Self {
        Self {
            customers: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn list(&self) -> Vec<Customer> {
        self.customers.lock().unwrap().clone()
    }

    fn get(&self, id: &str) -> Option<Customer> {
        self.customers
            .lock()
            .unwrap()
            .iter()
            .find(|c| c.id == id)
            .cloned()
    }

    fn create(&self, customer: Customer) {
        self.customers.lock().unwrap().push(customer);
    }
}

impl Default for CustomerRepository {
    fn default() -> Self {
        Self::new()
    }
}

struct CustomerController;

#[controller("/api/customers")]
impl CustomerController {
    #[route(GET, "/")]
    async fn list_customers(Inject(repo): Inject<CustomerRepository>) -> Json<Vec<Customer>> {
        Json(repo.list())
    }

    #[route(GET, "/:id")]
    async fn get_customer(
        axum::extract::Path(id): axum::extract::Path<String>,
        Inject(repo): Inject<CustomerRepository>,
    ) -> Result<Json<Customer>, StatusCode> {
        repo.get(&id).map(Json).ok_or(StatusCode::NOT_FOUND)
    }

    #[route(POST, "/")]
    async fn create_customer(
        Inject(repo): Inject<CustomerRepository>,
        ValidatedJson(body): ValidatedJson<CreateCustomerDto>,
    ) -> (StatusCode, Json<Customer>) {
        let customer = Customer {
            id: uuid::Uuid::new_v4().to_string(),
            name: body.name,
            email: body.email,
        };
        repo.create(customer.clone());
        (StatusCode::CREATED, Json(customer))
    }
}

#[tokio::main]
async fn main() {
    FragmentApp::new()
        .port(3000)
        .cors_origins(vec!["http://localhost:3001".into()])
        .start()
        .await;
}
