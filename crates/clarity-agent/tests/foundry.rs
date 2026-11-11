//! A Foundry connection: the company's Claude deployment on Microsoft
//! Foundry, read from the connection's fields.

use std::collections::HashMap;

use clarity_agent::foundry::{Foundry, SignedIn};
use clarity_auth::openid::{ANY_ORGANIZATION, ENTRA_AUTHORITY, Provider};

fn read(fields: &[(&str, &str)]) -> impl Fn(&str) -> Result<String, String> {
    let fields: HashMap<String, String> = fields.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
    move |key| Ok(fields.get(key).cloned().unwrap_or_default())
}

#[test]
fn the_resource_names_the_address_and_the_deployment_the_model() {
    let foundry = Foundry::of("work", read(&[("resource", "contoso-ai"), ("deployment", "claude-team"), ("clientId", "app-1")])).unwrap();
    assert_eq!(foundry.base_url, "https://contoso-ai.services.ai.azure.com/anthropic");
    assert_eq!(foundry.deployment, "claude-team");
    // No key: the person signs in with Microsoft, in any organization unless a tenant is named.
    let SignedIn::Microsoft(provider) = foundry.signed_in else { panic!("a Microsoft sign-in") };
    assert_eq!(provider, Provider::entra(ENTRA_AUTHORITY, ANY_ORGANIZATION, "app-1"));
}

#[test]
fn a_whole_address_a_tenant_or_a_key_may_be_given() {
    let foundry = Foundry::of("work", read(&[("baseUrl", "https://x.services.ai.azure.com/anthropic/"), ("deployment", "d"), ("tenant", "contoso.com"), ("clientId", "app-1")])).unwrap();
    assert_eq!(foundry.base_url, "https://x.services.ai.azure.com/anthropic");
    let SignedIn::Microsoft(provider) = foundry.signed_in else { panic!("a Microsoft sign-in") };
    assert_eq!(provider, Provider::entra(ENTRA_AUTHORITY, "contoso.com", "app-1"));
    let keyed = Foundry::of("work", read(&[("resource", "r"), ("deployment", "d"), ("apikey", "k-1")])).unwrap();
    assert!(matches!(keyed.signed_in, SignedIn::Key(key) if key == "k-1"));
}

#[test]
fn what_is_missing_is_said() {
    let said = |fields: &[(&str, &str)]| Foundry::of("work", read(fields)).err().unwrap();
    assert_eq!(said(&[("deployment", "d"), ("clientId", "a")]), "the connection work names no resource (or baseUrl) of Microsoft Foundry");
    assert_eq!(said(&[("resource", "r"), ("clientId", "a")]), "the connection work names no deployment");
    assert_eq!(said(&[("resource", "r"), ("deployment", "d")]), "the connection work has no apikey and no clientId to sign in with Microsoft");
    assert_eq!(said(&[("resource", "contoso ai/x"), ("deployment", "d"), ("clientId", "a")]), "the connection work has resource contoso ai/x; a Foundry resource's name has letters, digits and hyphens");
}
