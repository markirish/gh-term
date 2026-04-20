use std::process::Command;
use std::collections::HashMap;

use serde::Deserialize;

use super::{AuthBootstrap, AuthCheckResult, AuthError, AuthSession};

const DEFAULT_HOST: &str = "github.com";

#[derive(Debug, Clone)]
pub struct GhCliAuthBootstrap {
    host: String,
}

impl GhCliAuthBootstrap {
    pub fn new() -> GhCliAuthBootstrap {
        GhCliAuthBootstrap {
            host: DEFAULT_HOST.to_string(),
        }
    }

    pub fn with_host(host: impl Into<String>) -> GhCliAuthBootstrap {
        GhCliAuthBootstrap { host: host.into() }
    }

    fn run_gh(&self, args: &[&str]) -> Result<String, AuthError> {
        let output = Command::new("gh")
            .args(args)
            .output()
            .map_err(|err| {
                if err.kind() == std::io::ErrorKind::NotFound {
                    AuthError::GhNotInstalled
                } else {
                    AuthError::CommandFailed {
                        command: format!("gh {}", args.join(" ")),
                        details: err.to_string(),
                    }
                }
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let details = if !stderr.is_empty() { stderr } else { stdout };

            return Err(AuthError::CommandFailed {
                command: format!("gh {}", args.join(" ")),
                details,
            });
        }

        let stdout = String::from_utf8(output.stdout).map_err(|err| AuthError::InvalidResponse {
            command: format!("gh {}", args.join(" ")),
            details: err.to_string(),
        })?;

        Ok(stdout)
    }

    fn load_status(&self) -> Result<GhAuthStatusResponse, AuthError> {
        let stdout = self.run_gh(&[
            "auth",
            "status",
            "--hostname",
            &self.host,
            "--json",
            "hosts",
        ])?;

        serde_json::from_str(&stdout).map_err(|err| AuthError::InvalidResponse {
            command: "gh auth status --json hosts".to_string(),
            details: err.to_string(),
        })
    }

    fn find_active_account(
        &self,
        status: &GhAuthStatusResponse,
    ) -> Result<(String, bool), AuthError> {
        let host = status
            .hosts
            .get(&self.host)
            .ok_or_else(|| AuthError::NotAuthenticated {
                host: self.host.clone(),
                details: "No auth status was returned for this host.".to_string(),
            })?;

        let activeStatus = host
            .iter()
            .find(|a| a.active)
            .ok_or_else(|| AuthError::NotAuthenticated {
                host: self.host.clone(),
                details: "No active GitHub account found in gh auth status.".to_string(),
            })?;

        Ok((activeStatus.login.clone(), activeStatus.state == "success"))
    }

    fn get_token(&self) -> Result<String, AuthError> {
        let token = self
            .run_gh(&["auth", "token", "--hostname", &self.host])?
            .trim()
            .to_string();

        if token.is_empty() {
            return Err(AuthError::TokenUnavailable {
                host: self.host.clone(),
                details: "gh auth token returned an empty token.".to_string(),
            });
        }

        Ok(token)
    }
}

// impl Default for GhCliAuthBootstrap {
//     fn default() -> Self {
//         Self::new()
//     }
// }

impl AuthBootstrap for GhCliAuthBootstrap {
    fn check_auth(&self) -> Result<AuthCheckResult, AuthError> {
        let status = self.load_status()?;
        let (username, is_valid) = self.find_active_account(&status)?;

        if !is_valid {
            return Err(AuthError::NotAuthenticated {
                host: self.host.clone(),
                details: format!("Active account `{username}` is not in a valid auth state."),
            });
        }

        Ok(AuthCheckResult {
            host: self.host.clone(),
            username,
        })
    }

    fn get_session(&self) -> Result<AuthSession, AuthError> {
        let check = self.check_auth()?;
        let token = self.get_token()?;

        Ok(AuthSession {
            host: check.host,
            username: check.username,
            token,
        })
    }
}

#[derive(Debug, Deserialize)]
struct GhAuthStatusResponse {
    hosts: HashMap<String, Vec<GhAccountStatus>>,
}

#[derive(Debug, Deserialize)]
struct GhAccountStatus {
    state: String,
    active: bool,
    host: String,
    login: String,
    #[serde(rename = "tokenSource")]
    token_source: String,
    #[serde(rename = "gitProtocol")]
    git_protocol: String,
}