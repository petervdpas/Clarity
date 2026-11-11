//! A connection to Microsoft Foundry: the company's own Claude deployment,
//! behind Anthropic's Messages API ("https://<resource name>.services.ai.azure.com/anthropic",
//! Microsoft Learn, "Deploy and use Claude models in Microsoft Foundry"),
//! signed in with an API key or the person's Microsoft work account, through
//! Clarity.Auth's `openid` flow, the one generated apps sign in with too.

use clarity_auth::openid::{ANY_ORGANIZATION, ENTRA_AUTHORITY, Provider};

/// The sign-in flow, for the IDE that runs it.
pub use clarity_auth::openid;

/// The scope of Foundry's models ("Use scope https://ai.azure.com/.default").
pub const FOUNDRY_SCOPE: &str = "https://ai.azure.com/.default";

/// How a Foundry connection signs in.
pub enum SignedIn {
    /// The deployment's API key, sent as `x-api-key`.
    Key(String),
    /// The person's work account, through the browser; the refresh token is kept in the vault.
    Microsoft(Provider),
}

/// A Foundry connection, as its fields describe it.
pub struct Foundry {
    /// Where the Messages API is, without `/v1/messages`.
    pub base_url: String,
    /// The deployment's name, sent as the model; it "can be different from the model ID".
    pub deployment: String,
    pub signed_in: SignedIn,
}

impl Foundry {
    /// Reads the connection `name`'s fields through `field` (empty when a field is missing).
    pub fn of(name: &str, field: impl Fn(&str) -> Result<String, String>) -> Result<Foundry, String> {
        let base_url = field("baseUrl")?.trim().trim_end_matches('/').to_string();
        let base_url = if !base_url.is_empty() {
            base_url
        } else {
            let resource = field("resource")?.trim().to_string();
            if resource.is_empty() {
                return Err(format!("the connection {name} names no resource (or baseUrl) of Microsoft Foundry"));
            }
            if !resource.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
                return Err(format!("the connection {name} has resource {resource}; a Foundry resource's name has letters, digits and hyphens"));
            }
            format!("https://{resource}.services.ai.azure.com/anthropic")
        };
        let deployment = field("deployment")?.trim().to_string();
        if deployment.is_empty() {
            return Err(format!("the connection {name} names no deployment"));
        }
        let key = field("apikey")?.trim().to_string();
        let signed_in = if !key.is_empty() {
            SignedIn::Key(key)
        } else {
            let client_id = field("clientId")?.trim().to_string();
            if client_id.is_empty() {
                return Err(format!("the connection {name} has no apikey and no clientId to sign in with Microsoft"));
            }
            let tenant = Some(field("tenant")?.trim().to_string()).filter(|t| !t.is_empty()).unwrap_or_else(|| ANY_ORGANIZATION.into());
            SignedIn::Microsoft(Provider::entra(ENTRA_AUTHORITY, &tenant, &client_id))
        };
        Ok(Foundry { base_url, deployment, signed_in })
    }
}
