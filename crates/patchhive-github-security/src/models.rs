use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct GitHubDependabotAlert {
    #[serde(default)]
    pub number: u32,
    #[serde(default)]
    pub html_url: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default)]
    pub dependency: GitHubAlertDependency,
    #[serde(default)]
    pub security_advisory: GitHubSecurityAdvisory,
    #[serde(default)]
    pub security_vulnerability: GitHubSecurityVulnerability,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct GitHubAlertDependency {
    #[serde(default)]
    pub package: GitHubPackageRef,
    #[serde(default)]
    pub manifest_path: String,
    #[serde(default)]
    pub scope: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct GitHubSecurityAdvisory {
    #[serde(default)]
    pub ghsa_id: String,
    #[serde(default)]
    pub cve_id: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub severity: String,
    #[serde(default)]
    pub cwes: Vec<GitHubCweRef>,
    #[serde(default)]
    pub references: Vec<GitHubReference>,
    pub epss: Option<GitHubEpss>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct GitHubSecurityVulnerability {
    #[serde(default)]
    pub severity: String,
    #[serde(default)]
    pub vulnerable_version_range: String,
    #[serde(default)]
    pub package: GitHubPackageRef,
    pub first_patched_version: Option<GitHubPatchedVersion>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct GitHubPackageRef {
    #[serde(default)]
    pub ecosystem: String,
    #[serde(default)]
    pub name: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct GitHubPatchedVersion {
    #[serde(default)]
    pub identifier: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct GitHubCweRef {
    #[serde(default)]
    pub cwe_id: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct GitHubReference {
    #[serde(default)]
    pub url: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct GitHubEpss {
    #[serde(default)]
    pub percentage: f64,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct GitHubCodeScanningAlert {
    #[serde(default)]
    pub number: u32,
    #[serde(default)]
    pub html_url: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default)]
    pub rule: GitHubCodeRule,
    #[serde(default)]
    pub tool: GitHubCodeTool,
    #[serde(default)]
    pub most_recent_instance: GitHubCodeInstance,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct GitHubCodeRule {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub severity: String,
    #[serde(default)]
    pub security_severity_level: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct GitHubCodeTool {
    #[serde(default)]
    pub name: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct GitHubCodeInstance {
    #[serde(default, rename = "ref")]
    pub ref_: String,
    #[serde(default)]
    pub message: GitHubCodeMessage,
    #[serde(default)]
    pub location: GitHubCodeLocation,
    #[serde(default)]
    pub classifications: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct GitHubCodeMessage {
    #[serde(default)]
    pub text: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct GitHubCodeLocation {
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub start_line: u32,
}
