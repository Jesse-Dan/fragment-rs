use axum::body::Body;
use axum::response::Response;
use std::error::Error;

pub trait ExceptionFilter: Clone + Send + Sync + 'static {
    fn catch(&self, exception: &(dyn Error + Send + Sync), res: &mut Response<Body>);
}