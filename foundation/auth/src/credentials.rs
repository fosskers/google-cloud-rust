use base64::prelude::*;
use serde::Deserialize;
use tokio::fs;

use crate::error::Error;

const CREDENTIALS_FILE: &str = "application_default_credentials.json";

#[allow(dead_code)]
#[derive(Deserialize, Clone)]
pub struct ServiceAccountImpersonationInfo {
    pub(crate) token_lifetime_seconds: i32,
}

#[allow(dead_code)]
#[derive(Deserialize, Clone)]
pub struct ExecutableConfig {
    pub(crate) command: String,
    pub(crate) timeout_millis: Option<i32>,
    pub(crate) output_file: String,
}

#[allow(dead_code)]
#[derive(Deserialize, Clone)]
pub struct Format {
    #[serde(rename(deserialize = "type"))]
    pub(crate) tp: String,
    pub(crate) subject_token_field_name: String,
}

#[allow(dead_code)]
#[derive(Deserialize, Clone)]
pub struct CredentialSource {
    pub(crate) file: Option<String>,

    pub(crate) url: Option<String>,
    pub(crate) headers: Option<std::collections::HashMap<String, String>>,

    pub(crate) executable: Option<ExecutableConfig>,

    pub(crate) environment_id: Option<String>,
    pub(crate) region_url: Option<String>,
    pub(crate) regional_cred_verification_url: Option<String>,
    pub(crate) cred_verification_url: Option<String>,
    pub(crate) imdsv2_session_token_url: Option<String>,
    pub(crate) format: Option<Format>,
}

#[allow(dead_code)]
#[derive(Deserialize, Clone)]
pub struct CredentialsFile {
    #[serde(rename(deserialize = "type"))]
    pub tp: String,

    // Service Account fields
    pub client_email: Option<String>,
    pub private_key_id: Option<String>,
    pub private_key: Option<String>,
    pub auth_uri: Option<String>,
    pub token_uri: Option<String>,
    pub project_id: Option<String>,

    // User Credential fields
    // (These typically come from gcloud auth.)
    pub client_secret: Option<String>,
    pub client_id: Option<String>,
    pub refresh_token: Option<String>,

    // External Account fields
    pub audience: Option<String>,
    pub subject_token_type: Option<String>,
    #[serde(rename = "token_url")]
    pub token_url_external: Option<String>,
    pub token_info_url: Option<String>,
    pub service_account_impersonation_url: Option<String>,
    pub service_account_impersonation: Option<ServiceAccountImpersonationInfo>,
    pub delegates: Option<Vec<String>>,
    pub credential_source: Option<CredentialSource>,
    pub quota_project_id: Option<String>,
    pub workforce_pool_user_project: Option<String>,
}

impl CredentialsFile {
    pub async fn new() -> Result<Self, Error> {
        let credentials_json = {
            if let Ok(credentials) = Self::json_from_env().await {
                credentials
            } else {
                Self::json_from_file().await?
            }
        };

        Ok(serde_json::from_slice(credentials_json.as_slice())?)
    }

    pub async fn new_from_file(filepath: String) -> Result<Self, Error> {
        let credentials_json = fs::read(filepath).await?;
        Ok(serde_json::from_slice(credentials_json.as_slice())?)
    }

    async fn json_from_env() -> Result<Vec<u8>, ()> {
        let credentials = std::env::var("GOOGLE_APPLICATION_CREDENTIALS_JSON")
            .map_err(|_| ())
            .map(Vec::<u8>::from)?;

        if let Ok(decoded) = BASE64_STANDARD.decode(credentials.clone()) {
            Ok(decoded)
        } else {
            Ok(credentials)
        }
    }

    async fn json_from_file() -> Result<Vec<u8>, Error> {
        let path = match std::env::var("GOOGLE_APPLICATION_CREDENTIALS") {
            Ok(s) => Ok(std::path::Path::new(s.as_str()).to_path_buf()),
            Err(_e) => {
                // get well known file name
                if cfg!(target_os = "windows") {
                    let app_data = std::env::var("APPDATA")?;
                    Ok(std::path::Path::new(app_data.as_str())
                        .join("gcloud")
                        .join(CREDENTIALS_FILE))
                } else {
                    match home::home_dir() {
                        Some(s) => Ok(s.join(".config").join("gcloud").join(CREDENTIALS_FILE)),
                        None => Err(Error::NoHomeDirectoryFound),
                    }
                }
            }
        }?;

        let credentials_json = fs::read(path).await?;

        Ok(credentials_json)
    }

    pub(crate) fn try_to_private_key(&self) -> Result<jsonwebtoken::EncodingKey, Error> {
        match self.private_key.as_ref() {
            Some(key) => Ok(jsonwebtoken::EncodingKey::from_rsa_pem(key.as_bytes())?),
            None => Err(Error::NoPrivateKeyFound),
        }
    }
}
