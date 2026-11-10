use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::RwLock;

pub trait WorkflowStep: Send + Sync {
    fn name(&self) -> &str;
    fn retries(&self) -> u32;
    fn execute(&self, input: serde_json::Value) -> Pin<Box<dyn Future<Output = Result<serde_json::Value, String>> + Send + '_>>;
}

#[derive(Clone)]
pub struct WorkflowState {
    pub id: String,
    pub workflow: String,
    pub input: serde_json::Value,
    pub completed_steps: Vec<String>,
    pub last_result: Option<serde_json::Value>,
    pub status: WorkflowStatus,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowStatus {
    Running,
    Completed,
    Failed,
}

pub struct WorkflowDef<S> {
    pub name: String,
    steps: Vec<Arc<S>>,
    state_store: Arc<dyn WorkflowStateStore + Send + Sync>,
}

impl<S: WorkflowStep> WorkflowDef<S> {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            steps: Vec::new(),
            state_store: Arc::new(InMemoryStateStore::default()),
        }
    }

    pub fn step(mut self, step: S) -> Self {
        self.steps.push(Arc::new(step));
        self
    }

    pub fn with_state_store(mut self, store: Arc<dyn WorkflowStateStore + Send + Sync>) -> Self {
        self.state_store = store;
        self
    }
}

pub trait WorkflowStateStore: Send + Sync {
    fn save(&self, state: &WorkflowState) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + '_>>;
    fn load(&self, id: &str) -> Pin<Box<dyn Future<Output = Option<WorkflowState>> + Send + '_>>;
}

#[derive(Default)]
pub struct InMemoryStateStore {
    states: RwLock<HashMap<String, WorkflowState>>,
}

impl WorkflowStateStore for InMemoryStateStore {
    fn save(&self, state: &WorkflowState) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + '_>> {
        let state = state.clone();
        Box::pin(async move {
            self.states.write().await.insert(state.id.clone(), state);
            Ok(())
        })
    }

    fn load(&self, id: &str) -> Pin<Box<dyn Future<Output = Option<WorkflowState>> + Send + '_>> {
        let id = id.to_string();
        Box::pin(async move {
            self.states.read().await.get(&id).cloned()
        })
    }
}

pub struct WorkflowRunner;

impl WorkflowRunner {
    pub async fn run<S>(workflow: &WorkflowDef<S>, input: serde_json::Value) -> Result<serde_json::Value, String>
    where
        S: WorkflowStep,
    {
        let id = format!("{}:{}", workflow.name, chrono::Utc::now().timestamp_millis());
        let mut state = WorkflowState {
            id: id.clone(),
            workflow: workflow.name.clone(),
            input: input.clone(),
            completed_steps: Vec::new(),
            last_result: None,
            status: WorkflowStatus::Running,
            error: None,
        };

        workflow.state_store.save(&state).await?;

        let mut current = input;
        for step in &workflow.steps {
            let step_name = step.name().to_string();

            if state.completed_steps.contains(&step_name) {
                continue;
            }

            let mut attempts = 0;
            let max_retries = step.retries();

            loop {
                match step.execute(current.clone()).await {
                    Ok(result) => {
                        state.completed_steps.push(step_name.clone());
                        state.last_result = Some(result.clone());
                        current = result;
                        break;
                    }
                    Err(e) => {
                        attempts += 1;
                        if attempts > max_retries {
                            state.status = WorkflowStatus::Failed;
                            state.error = Some(e.clone());
                            workflow.state_store.save(&state).await?;
                            return Err(e);
                        }
                    }
                }
            }

            workflow.state_store.save(&state).await?;
        }

        state.status = WorkflowStatus::Completed;
        workflow.state_store.save(&state).await?;
        Ok(state.last_result.unwrap_or(serde_json::Value::Null))
    }

    pub async fn resume<S>(workflow: &WorkflowDef<S>, execution_id: &str) -> Result<serde_json::Value, String>
    where
        S: WorkflowStep,
    {
        let state = workflow.state_store.load(execution_id).await;
        match state {
            Some(s) if s.status == WorkflowStatus::Failed => {
                let mut ws = s.clone();
                ws.status = WorkflowStatus::Running;
                ws.error = None;
                let mut current = ws.last_result.clone().unwrap_or(serde_json::Value::Null);

                for step in &workflow.steps {
                    let step_name = step.name().to_string();
                    if ws.completed_steps.contains(&step_name) {
                        continue;
                    }

                    match step.execute(current.clone()).await {
                        Ok(result) => {
                            ws.completed_steps.push(step_name.clone());
                            ws.last_result = Some(result.clone());
                            current = result;
                        }
                        Err(e) => {
                            ws.status = WorkflowStatus::Failed;
                            ws.error = Some(e.clone());
                            workflow.state_store.save(&ws).await?;
                            return Err(e);
                        }
                    }
                    workflow.state_store.save(&ws).await?;
                }

                ws.status = WorkflowStatus::Completed;
                workflow.state_store.save(&ws).await?;
                Ok(ws.last_result.unwrap_or(serde_json::Value::Null))
            }
            _ => Err(format!("Workflow state not found or not failed: {}", execution_id)),
        }
    }
}

pub mod prelude {
    pub use super::{WorkflowDef, WorkflowRunner, WorkflowState, WorkflowStateStore, WorkflowStep, WorkflowStatus};
}