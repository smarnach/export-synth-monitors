use reqwest::Result;
use serde::de::DeserializeOwned;
use serde_json::json;

#[derive(Clone, Debug)]
pub struct Client<'a> {
    api_key: &'a str,
    client: reqwest::Client,
}

impl<'a> Client<'a> {
    pub fn new(api_key: &'a str) -> Self {
        let client = reqwest::Client::new();
        Self { api_key, client }
    }

    pub async fn query<T>(&self, query: &str, variables: &str) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let response = self
            .client
            .post("https://api.newrelic.com/graphql")
            .header("API-Key", self.api_key)
            .json(&json!({"query": query, "variables": variables}))
            .send()
            .await?;
        response.json().await
    }
}
