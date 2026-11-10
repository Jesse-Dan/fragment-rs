use reqwest::Client;
use std::collections::HashMap;
use std::time::Duration;

pub struct FrgClient {
    client: Client,
    base_url: Option<String>,
    max_retries: u32,
    headers: HashMap<String, String>,
}

impl FrgClient {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .build()
                .expect("Failed to create HTTP client"),
            base_url: None,
            max_retries: 0,
            headers: HashMap::new(),
        }
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    pub fn with_max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.client = Client::builder()
            .timeout(timeout)
            .build()
            .expect("Failed to create HTTP client");
        self
    }

    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(name.into(), value.into());
        self
    }

    fn resolve_url(&self, path: &str) -> String {
        match &self.base_url {
            Some(base) => format!(
                "{}/{}",
                base.trim_end_matches('/'),
                path.trim_start_matches('/')
            ),
            None => path.to_string(),
        }
    }

    fn apply_headers(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        let mut builder = req;
        for (k, v) in &self.headers {
            builder = builder.header(k.as_str(), v.as_str());
        }
        builder
    }

    pub async fn get(&self, path: &str) -> Result<reqwest::Response, reqwest::Error> {
        let url = self.resolve_url(path);
        let mut last_err = None;

        for attempt in 0..=self.max_retries {
            let req = self.apply_headers(self.client.get(&url));
            match req.send().await {
                Ok(resp) => return Ok(resp),
                Err(e) => {
                    if attempt < self.max_retries {
                        tokio::time::sleep(Duration::from_millis(
                            100 * 2u64.saturating_pow(attempt as u32),
                        ))
                        .await;
                    }
                    last_err = Some(e);
                }
            }
        }
        Err(last_err.unwrap())
    }

    pub async fn post<T: serde::Serialize>(
        &self,
        path: &str,
        body: &T,
    ) -> Result<reqwest::Response, reqwest::Error> {
        let url = self.resolve_url(path);
        self.apply_headers(self.client.post(&url))
            .json(body)
            .send()
            .await
    }

    pub async fn put<T: serde::Serialize>(
        &self,
        path: &str,
        body: &T,
    ) -> Result<reqwest::Response, reqwest::Error> {
        let url = self.resolve_url(path);
        self.apply_headers(self.client.put(&url))
            .json(body)
            .send()
            .await
    }

    pub async fn delete(&self, path: &str) -> Result<reqwest::Response, reqwest::Error> {
        let url = self.resolve_url(path);
        self.apply_headers(self.client.delete(&url)).send().await
    }

    pub async fn patch<T: serde::Serialize>(
        &self,
        path: &str,
        body: &T,
    ) -> Result<reqwest::Response, reqwest::Error> {
        let url = self.resolve_url(path);
        self.apply_headers(self.client.patch(&url))
            .json(body)
            .send()
            .await
    }
}

impl Default for FrgClient {
    fn default() -> Self {
        Self::new()
    }
}
