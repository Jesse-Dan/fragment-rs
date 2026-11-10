use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum JobStatus {
    Scheduled,
    Running,
    Completed,
    Failed,
}

pub trait Job: Send + Sync + 'static {
    fn name(&self) -> &str;
    fn schedule(&self) -> &str;
    fn run(self: Arc<Self>) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>>
    where
        Self: Send;
}

pub struct JobScheduler {
    jobs: HashMap<String, Arc<dyn Job>>,
}

impl JobScheduler {
    pub fn register(&mut self, job: Arc<dyn Job>) {
        self.jobs.insert(job.name().to_string(), job);
    }

    pub fn get(&self, name: &str) -> Option<&Arc<dyn Job>> {
        self.jobs.get(name)
    }

    pub fn list(&self) -> Vec<&str> {
        self.jobs.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for JobScheduler {
    fn default() -> Self {
        Self {
            jobs: HashMap::new(),
        }
    }
}
