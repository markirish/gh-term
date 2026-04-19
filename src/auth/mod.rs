pub mod gh_cli;

pub use gh_cli::GhCliAuthBootstrap;

#[derive(Debug, Clone)]
pub struct AuthSession {
  pub host: String,
  pub username: String,
  pub token: String,
}

#[derive(Debug, Clone)]
pub struct AuthCheckResult {
  pub host: String,
  pub username: String,
}

#[derive(Debug, Clone)]
pub enum AuthError {
    GhNotInstalled,
    NotAuthenticated { host: String, details: String },
    TokenUnavailable { host: String, details: String },
    CommandFailed    { command: String, details: String },
    InvalidResponse  { command: String, details: String },
}

impl std::fmt::Display for AuthError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result {
    match self {
      Self::GhNotInstalled => write!(f, "GitHub CLI (`gh`) is not installed or not on PATH."),
      Self::NotAuthenticated { host, details } => {
        write!(f, "Not authenticated with host `{host}`: {details}")
      }
      Self::TokenUnavailable { host, details } => {
        write!(f, "Could not retrieve token for host `{host}`: {details}")
      }
      Self::CommandFailed { command, details } => {
        write!(f, "Command `{command}` failed: {details}")
      }
      Self::InvalidResponse { command, details } => {
        write!(f, "Command `{command}` returned an unexpected response: {details}")
      }
    }
  }
}

impl std::error::Error for AuthError {}

pub trait AuthBootstrap {
    fn check_auth(&self) -> Result<AuthCheckResult, AuthError>;
    fn get_session(&self) -> Result<AuthSession, AuthError>;
}