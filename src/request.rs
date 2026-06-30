use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use clap::ValueEnum;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::stats::RequestResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize, Default)]
#[serde(rename_all = "UPPERCASE")]
#[clap(rename_all = "uppercase")]
pub enum HttpMethod {
    #[default]
    Get,
    Post,
    Put,
    Delete,
}

impl HttpMethod {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Delete => "DELETE",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestSpec {
    pub url: String,
    pub method: HttpMethod,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
    pub timeout_secs: u64,
}

impl RequestSpec {
    pub fn validate(&self) -> Result<()> {
        reqwest::Url::parse(&self.url)
            .with_context(|| format!("invalid URL: {}", self.url))?;

        if matches!(self.method, HttpMethod::Get | HttpMethod::Delete) && self.body.is_some() {
            bail!("body is not supported for {} requests", self.method.as_str());
        }

        Ok(())
    }
}

pub fn parse_header(raw: &str) -> Result<(String, String)> {
    let Some((key, value)) = raw.split_once(':') else {
        bail!("invalid header format '{raw}': expected 'Key: Value'");
    };

    let key = key.trim();
    let value = value.trim();

    if key.is_empty() {
        bail!("invalid header format '{raw}': header name cannot be empty");
    }

    Ok((key.to_string(), value.to_string()))
}

pub fn parse_headers(raw_headers: &[String]) -> Result<Vec<(String, String)>> {
    raw_headers.iter().map(|h| parse_header(h)).collect()
}

pub async fn build_client(timeout_secs: u64) -> Result<Client> {
    Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .build()
        .context("failed to build HTTP client")
}

pub async fn send_request(client: &Client, spec: &RequestSpec) -> RequestResult {
    let start = Instant::now();

    let mut builder = match spec.method {
        HttpMethod::Get => client.get(&spec.url),
        HttpMethod::Post => client.post(&spec.url),
        HttpMethod::Put => client.put(&spec.url),
        HttpMethod::Delete => client.delete(&spec.url),
    };

    for (key, value) in &spec.headers {
        builder = builder.header(key.as_str(), value.as_str());
    }

    if let Some(body) = &spec.body {
        builder = builder.body(body.clone());
    }

    match builder.send().await {
        Ok(response) => RequestResult {
            latency: start.elapsed(),
            status: Some(response.status().as_u16()),
            error: None,
        },
        Err(err) => RequestResult {
            latency: start.elapsed(),
            status: None,
            error: Some(classify_error(&err)),
        },
    }
}

pub fn classify_error(err: &reqwest::Error) -> String {
    if err.is_timeout() {
        "timeout".to_string()
    } else if err.is_connect() {
        "connection error".to_string()
    } else if err.is_request() {
        "request error".to_string()
    } else {
        "network error".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_header_valid() {
        let (k, v) = parse_header("Authorization: Bearer token").unwrap();
        assert_eq!(k, "Authorization");
        assert_eq!(v, "Bearer token");
    }

    #[test]
    fn parse_header_missing_colon() {
        assert!(parse_header("Authorization Bearer").is_err());
    }

    #[test]
    fn parse_header_empty_key() {
        assert!(parse_header(": value").is_err());
    }

    #[test]
    fn validate_rejects_body_on_get() {
        let spec = RequestSpec {
            url: "https://example.com".to_string(),
            method: HttpMethod::Get,
            headers: vec![],
            body: Some("{}".to_string()),
            timeout_secs: 10,
        };
        assert!(spec.validate().is_err());
    }

    #[test]
    fn validate_accepts_post_with_body() {
        let spec = RequestSpec {
            url: "https://example.com".to_string(),
            method: HttpMethod::Post,
            headers: vec![],
            body: Some("{}".to_string()),
            timeout_secs: 10,
        };
        assert!(spec.validate().is_ok());
    }

    #[tokio::test]
    async fn classify_timeout_error() {
        use std::time::Duration;

        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(2)))
            .mount(&server)
            .await;

        let client = build_client(1).await.unwrap();
        let spec = RequestSpec {
            url: server.uri(),
            method: HttpMethod::Get,
            headers: vec![],
            body: None,
            timeout_secs: 1,
        };

        let result = send_request(&client, &spec).await;
        assert_eq!(result.error.as_deref(), Some("timeout"));
    }
}
