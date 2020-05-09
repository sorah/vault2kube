use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::error::Error;

const K8S_INCLUSTER_SERVICE_TOKENFILE: &str = "/var/run/secrets/kubernetes.io/serviceaccount/token";

#[derive(Debug, Clone)]
pub struct Client {
    addr: reqwest::Url,
    token: Option<String>,
    http: reqwest::Client,
}

impl Client {
    pub async fn new() -> anyhow::Result<Self> {
        let addr_str = env::var("VAULT_ADDR")
            .ok()
            .ok_or(anyhow!("VAULT_ADDR must be present"))?;
        let addr = reqwest::Url::parse(&addr_str)?.join("v1/")?;
        let namespace = env::var("VAULT_NAMESPACE").ok();
        let cacert = match env::var("VAULT_CACERT") {
            Ok(path) => {
                let pem = std::fs::read(path)?;
                Some(reqwest::Certificate::from_pem(&pem)?)
            }
            Err(_e) => None,
        };
        let token = env::var("VAULT_TOKEN").ok();
        let token_given = token.is_some();

        let mut http = reqwest::Client::builder();
        let mut http_headers = reqwest::header::HeaderMap::new();
        http_headers.insert(
            "X-Vault-Request",
            reqwest::header::HeaderValue::from_static("true"),
        );
        if let Some(ns) = namespace {
            http_headers.insert(
                "X-Vault-Namespace",
                reqwest::header::HeaderValue::from_str(&ns)?,
            );
        }
        http = http.default_headers(http_headers);
        if let Some(c) = cacert {
            http = http.add_root_certificate(c);
        }

        let mut client = Client {
            addr,
            token,
            http: http.build()?,
        };
        if !token_given {
            let k8s_jwt_path = env::var("VAULT_K8S_TOKEN_PATH")
                .unwrap_or(K8S_INCLUSTER_SERVICE_TOKENFILE.to_string());
            let k8s_jwt = std::fs::read(k8s_jwt_path)?;
            client.authenticate_using_kubernetes(
			   env::var("VAULT_K8S_PATH").ok().ok_or(
				   anyhow!("VAULT_K8S_PATH, VAULT_K8S_ROLE must be present when no VAULT_TOKEN is present"),
			   )?,
			   env::var("VAULT_K8S_ROLE").ok().ok_or(
				   anyhow!("VAULT_K8S_PATH, VAULT_K8S_ROLE must be present when no VAULT_TOKEN is present"),
			   )?,
			   String::from_utf8(k8s_jwt)?,
			).await?;
        };
        Ok(client)
    }

    pub async fn read(&self, path: &str) -> Result<LeaseResponse, Box<dyn Error>> {
        let response: LeaseResponse = self.get(path).await?.json().await?;
        Ok(response)
    }

    pub async fn renew(&self, lease_id: &str) -> Result<RenewResponse, Box<dyn Error>> {
        let payload = RenewRequest {
            lease_id: lease_id.to_string(),
        };
        let response: RenewResponse = self
            .post("sys/leases/renew", &payload)
            .await?
            .json()
            .await?;
        Ok(response)
    }

    pub async fn revoke(&self, lease_id: &str) -> Result<reqwest::Response, Box<dyn Error>> {
        let payload = RenewRequest {
            lease_id: lease_id.to_string(),
        };
        let response = self.post("sys/leases/revoke", &payload).await?;
        Ok(response)
    }

    async fn authenticate_using_kubernetes(
        &mut self,
        path: String,
        role: String,
        jwt: String,
    ) -> anyhow::Result<()> {
        let login_path = format!("{}/login", path);
        let payload = KubernetesAuthRequest { role, jwt };
        let response: AuthResponse = self.post(&login_path, &payload).await?.json().await?;
        self.token = Some(response.auth.client_token);
        Ok(())
    }

    async fn get(&self, path: &str) -> Result<reqwest::Response, reqwest::Error> {
        let url = self.addr.join(&path).unwrap();
        let request = self.http.get(url);
        self.send(request).await
    }

    async fn post<T: Serialize + ?Sized>(
        &self,
        path: &str,
        payload: &T,
    ) -> Result<reqwest::Response, reqwest::Error> {
        let url = self.addr.join(&path).unwrap();
        let request = self.http.post(url).json(&payload);
        self.send(request).await
    }

    async fn send(
        &self,
        mut request: reqwest::RequestBuilder,
    ) -> Result<reqwest::Response, reqwest::Error> {
        if let Some(token) = &self.token {
            request = request.header("X-Vault-Token", token.to_owned());
        }
        let req = request.build()?;
        self.http.execute(req).await
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct KubernetesAuthRequest {
    jwt: String,
    role: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct AuthResponse {
    auth: AuthResponseData,
}

#[derive(Debug, Serialize, Deserialize)]
struct AuthResponseData {
    pub client_token: String,
    pub accessor: String,
    pub policies: Vec<String>,
    pub lease_duration: u32,
    pub renewable: bool,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct RenewRequest {
    pub lease_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LeaseResponse {
    pub request_id: String,
    pub lease_id: String,
    pub lease_duration: u32,
    pub renewable: bool,
    pub data: HashMap<String, String>,
    pub warnings: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RenewResponse {
    pub request_id: String,
    pub lease_id: String,
    pub lease_duration: u32,
    pub renewable: bool,
    pub warnings: Option<Vec<String>>,
}
