
use std::io::Read;

use anyhow::{Result, Context, anyhow};
use log::info;
use serde::Deserialize;
use ureq::post;

const META_GRAPH_BASE_URL: &str = "https://meta.graph.meta.com";
const OCULUS_GRAPH_BASE_URL: &str = "https://graph.oculus.com";
const OCULUS_BINARY_DOWNLOAD_URL: &str = "https://securecdn.oculus.com/binaries/download/";

fn extract_access_token(access_token_result: &serde_json::Value) -> Result<String> {
    Ok(access_token_result["access_token"]
        .as_str().ok_or(anyhow!("access_token was not a string"))?.to_string())
}

pub fn meta_accounts_login(email_addr: &str, password: &str) -> Result<String> {
    let resp = crate::external_res::get_agent().post(&format!("{META_GRAPH_BASE_URL}/accounts_login")).send_form(&[
        ("access_token", "FRL|778542610035039|2e189079414d3a6e5642a789322b1940"),
        ("contact_point_type", "EMAIL_ADDRESS"),
        ("contact_point", email_addr),
        ("password", password)
    ]).context("Failed to send accounts login POST")?;

    let access_token_result: serde_json::Value = serde_json::from_reader(resp.into_reader())?;
    extract_access_token(&access_token_result)
}

fn get_horizon_access_token(access_token: &str) -> Result<String> {
    let resp = crate::external_res::get_agent().post(&format!("{META_GRAPH_BASE_URL}/graphql"))
        .send_form(&[
            ("access_token", access_token),
            ("variables", "{\"app_id\":\"1582076955407037\"}"),
            ("doc_id", "5787825127910775")
        ]).context("Failed to send get access token POST")?;

    let json_doc: serde_json::Value = serde_json::from_reader(resp.into_reader())?;

    // Extract the horizon access token from the document.
    let access_token_result = &json_doc["data"]
        ["xfr_create_profile_token"]
        ["profile_tokens"]
        .as_array().ok_or(anyhow!("profile_tokens was not an array!"))?[0];
    
    extract_access_token(access_token_result)

}

// Authenticates as the given application ID, and returns the authenticated access token.
fn authenticate_application(access_token: &str, app_id: u64) -> Result<String> {
    let resp = crate::external_res::get_agent().post(&format!("{OCULUS_GRAPH_BASE_URL}/authenticate_application"))
        .send_form(&[
            ("access_token", access_token),
            ("app_id", &app_id.to_string())
        ]).context("Failed to send authenticate_application POST")?;

    let access_token_result: serde_json::Value = serde_json::from_reader(resp.into_reader())?;
    extract_access_token(&access_token_result)
}

pub fn get_quest_access_token(email: &str, password: &str) -> Result<String> {
    let access_token = meta_accounts_login(email, password).context("Accounts login failed")?;
    info!("Account login succeeded");
    let horizon_access_token = get_horizon_access_token(&access_token).context("Failed to get horizon token")?;
    info!("Successfully obtained horizon access token");
    let quest_access_token = authenticate_application(&horizon_access_token, 1481000308606657)
        .context("Failed to authenticate application")?;

    Ok(quest_access_token)
}

#[derive(Deserialize)]
pub struct ResponseData<T> {
    pub data: Node<T>
}

#[derive(Deserialize, Debug)]
pub struct Nodes<T> {
    pub nodes: Vec<T>
}

#[derive(Deserialize, Debug)]
pub struct Node<T> {
    pub node: T
}

#[derive(Deserialize, Debug)]
pub struct ReleaseChannel {
    pub channel_name: String,
    pub id: String
}

#[derive(Deserialize, Debug)]
pub struct AndroidBinary {
    pub version: String,
    pub version_code: u32,
    pub binary_release_channels: Nodes<ReleaseChannel>,
    pub id: String,
    // Not included in list of all versions but only included if we get the details about this particular binary
    pub obb_binary: Option<ObbBinary>
}

#[derive(Deserialize, Debug)]
pub struct ObbBinary {
    pub id: String,
    pub file_name: String
}

#[derive(Deserialize, Debug)]
pub struct Application {
    pub primary_binaries: Nodes<AndroidBinary>
}

// Lists all of the available versions of the given app ID
pub fn list_app_versions(access_token: &str, app_id: &str) -> Result<Vec<AndroidBinary>> {
    let resp = crate::external_res::get_agent().post(&format!("{OCULUS_GRAPH_BASE_URL}/graphql"))
        .send_form(&[
            ("access_token", access_token),
            ("doc_id", "2885322071572384"),
            ("variables", &format!("{{\"applicationID\":\"{app_id}\"}}"))
        ])?;

    let string = resp.into_string()?;

    let req_result: ResponseData<Application> = serde_json::from_str(&string)?;

    Ok(req_result.data.node.primary_binaries.nodes)
}

// Gets to corresponding obb binary for the given android binary, if there is one.
pub fn get_obb_binary(access_token: &str, android_binary_id: &str) -> Result<Option<ObbBinary>> {
    let resp = crate::external_res::get_agent().post(&format!("{OCULUS_GRAPH_BASE_URL}/graphql"))
    .send_form(&[
        ("access_token", access_token),
        ("doc_id", "24072064135771905"),
        ("variables", &format!("{{\"binaryID\":\"{android_binary_id}\"}}"))
    ])?;

    let string = resp.into_string()?;

    let req_result: ResponseData<AndroidBinary> = serde_json::from_str(&string)?;
    Ok(req_result.data.node.obb_binary)
}

// Starts a request to download the binary with the given binary ID.
pub fn download_binary(access_token: &str, binary_id: &str) -> Result<Box<dyn Read>> {
    Ok(crate::external_res::get_agent().get(OCULUS_BINARY_DOWNLOAD_URL)
        .query("access_token", access_token)
        .query("id", binary_id)
        .call()?
        .into_reader())
}