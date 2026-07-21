//! Manual (offline) bank transfer operations.
//!
//! Ports the PHP `Actions\ManualBankTransfer` trait. These endpoints live on a
//! different host to the versioned API and use `multipart/form-data` (create)
//! or `application/x-www-form-urlencoded` (status update).

use std::collections::HashMap;

use reqwest::multipart::{Form, Part};
use serde_json::Value;

use crate::client::Bayarcash;
use crate::error::{Error, Result};
use crate::models::ManualBankTransferResponse;
use crate::requests::ManualBankTransferRequest;

impl Bayarcash {
    /// The manual-transfer base URL (differs from the versioned API host).
    fn manual_transfer_base_url(&self) -> &'static str {
        if self.sandbox {
            "https://console.bayarcash-sandbox.com/api"
        } else {
            "https://console.bayar.cash/api"
        }
    }

    /// Submit a manual bank transfer with proof of payment
    /// (`post /manual-bank-transfer`).
    ///
    /// `allow_redirect` controls whether HTTP redirects are followed (mirrors
    /// the PHP `$allowRedirect` argument; default `false`).
    pub async fn create_manual_bank_transfer(
        &self,
        request: &ManualBankTransferRequest,
        allow_redirect: bool,
    ) -> Result<ManualBankTransferResponse> {
        validate_manual_transfer(request)?;

        let mut form = Form::new();
        for (key, value) in request.text_fields() {
            form = form.text(key, value);
        }

        // bank_transfer_date defaults to today (Y-m-d) when omitted.
        let bank_transfer_date = request
            .bank_transfer_date
            .clone()
            .unwrap_or_else(|| chrono::Local::now().format("%Y-%m-%d").to_string());
        form = form.text("bank_transfer_date", bank_transfer_date);

        if let Some(path) = &request.proof_of_payment {
            let bytes = std::fs::read(path)?;
            let file_name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "proof_of_payment".to_string());
            let mime = file_content_type(path);
            let part = Part::bytes(bytes)
                .file_name(file_name)
                .mime_str(&mime)
                .map_err(Error::from)?;
            form = form.part("proof_of_payment", part);
        }

        let client = reqwest::Client::builder()
            .redirect(if allow_redirect {
                reqwest::redirect::Policy::limited(10)
            } else {
                reqwest::redirect::Policy::none()
            })
            .build()
            .map_err(Error::from)?;

        let url = format!("{}/manual-bank-transfer", self.manual_transfer_base_url());
        let response = client
            .post(&url)
            .header(reqwest::header::ACCEPT, "application/json")
            .bearer_auth(&self.token)
            .timeout(self.timeout)
            .multipart(form)
            .send()
            .await?;

        let status = response.status().as_u16();
        let location = response
            .headers()
            .get(reqwest::header::LOCATION)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let body = response.text().await?;

        process_manual_transfer_response(body, status, allow_redirect, location)
    }

    /// Update the status of an existing manual bank transfer
    /// (`post /manual-bank-transfer/update-status`).
    pub async fn update_manual_bank_transfer_status(
        &self,
        ref_no: &str,
        status: &str,
        amount: &str,
    ) -> Result<Value> {
        let url = format!(
            "{}/manual-bank-transfer/update-status",
            self.manual_transfer_base_url()
        );
        let form = [
            ("ref_no", ref_no),
            ("status", status),
            ("amount", amount),
        ];
        let response = self
            .http
            .post(&url)
            .header(reqwest::header::ACCEPT, "application/json")
            .bearer_auth(&self.token)
            .timeout(self.timeout)
            .form(&form)
            .send()
            .await?;

        let http_code = response.status().as_u16();
        let body = response.text().await?;

        if (200..300).contains(&http_code) {
            Ok(serde_json::from_str::<Value>(&body).unwrap_or(Value::String(body)))
        } else {
            Err(manual_transfer_error(&body, http_code))
        }
    }
}

fn validate_manual_transfer(request: &ManualBankTransferRequest) -> Result<()> {
    if request.payment_gateway != 2 {
        return Err(Error::InvalidArgument(
            "Invalid payment gateway. Value must be 2 for manual bank transfers.".to_string(),
        ));
    }
    if let Some(path) = &request.proof_of_payment {
        if !path.exists() {
            return Err(Error::InvalidArgument(
                "Proof of payment file does not exist".to_string(),
            ));
        }
    }
    Ok(())
}

fn process_manual_transfer_response(
    body: String,
    http_code: u16,
    allow_redirect: bool,
    location: Option<String>,
) -> Result<ManualBankTransferResponse> {
    if (200..300).contains(&http_code) {
        if body.contains("<form") {
            let form_data = parse_manual_bank_transfer_response(&body);
            let return_url = form_data.get("return_url").cloned();
            Ok(ManualBankTransferResponse::HtmlForm {
                html_form: body,
                form_data,
                return_url,
            })
        } else if let Ok(value) = serde_json::from_str::<Value>(&body) {
            Ok(ManualBankTransferResponse::Json(value))
        } else {
            Ok(ManualBankTransferResponse::Raw(body))
        }
    } else if (300..400).contains(&http_code) && !allow_redirect {
        Ok(ManualBankTransferResponse::Redirect {
            redirect_url: location.unwrap_or(body),
        })
    } else {
        Err(manual_transfer_error(&body, http_code))
    }
}

/// Mirror PHP `handleApiError`: use a JSON `message` if present, else a generic
/// message with the truncated body.
fn manual_transfer_error(body: &str, http_code: u16) -> Error {
    if let Ok(Value::Object(map)) = serde_json::from_str::<Value>(body) {
        if let Some(Value::String(message)) = map.get("message") {
            return Error::FailedAction(message.clone());
        }
    }
    let snippet: String = body.chars().take(200).collect();
    Error::Api {
        status: http_code,
        body: snippet,
    }
}

/// Extract structured data from the auto-submitting HTML form response.
/// Port of PHP `parseManualBankTransferResponse`.
pub(crate) fn parse_manual_bank_transfer_response(html: &str) -> HashMap<String, String> {
    let mut data = HashMap::new();

    if let Some(form_id) = first_quoted_value(html, "id=\"") {
        data.insert("form_id".to_string(), form_id);
    }
    if let Some(action) = first_quoted_value(html, "action=\"") {
        data.insert("return_url".to_string(), action);
    }
    for (name, value) in hidden_inputs(html) {
        data.insert(name, value);
    }

    data
}

/// Find `needle` (e.g. `id="`) and return the text up to the next `"`.
fn first_quoted_value(html: &str, needle: &str) -> Option<String> {
    let start = html.find(needle)? + needle.len();
    let rest = &html[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// Extract all `<input name="X" type="hidden" value="Y">` pairs.
fn hidden_inputs(html: &str) -> Vec<(String, String)> {
    const OPEN: &str = "<input name=\"";
    const MID: &str = "\" type=\"hidden\" value=\"";
    let mut out = Vec::new();
    let mut cursor = 0;
    while let Some(rel) = html[cursor..].find(OPEN) {
        let name_start = cursor + rel + OPEN.len();
        let rest = &html[name_start..];
        let Some(name_end) = rest.find('"') else {
            break;
        };
        let name = rest[..name_end].to_string();
        let after_name = &rest[name_end..];
        if let Some(stripped) = after_name.strip_prefix(MID) {
            if let Some(value_end) = stripped.find('"') {
                let value = stripped[..value_end].to_string();
                out.push((name, value));
                cursor = name_start + name_end + MID.len() + value_end;
                continue;
            }
        }
        cursor = name_start + name_end;
    }
    out
}

/// MIME type from a file extension. Port of PHP `getFileContentType`.
fn file_content_type(path: &std::path::Path) -> String {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "pdf" => "application/pdf",
        _ => "application/octet-stream",
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_html_form() {
        let html = r#"<form id="pay_form" method="post" action="https://bank.example/submit">
            <input name="token" type="hidden" value="abc123">
            <input name="ref" type="hidden" value="MT-1001">
        </form>"#;
        let data = parse_manual_bank_transfer_response(html);
        assert_eq!(data.get("form_id").map(String::as_str), Some("pay_form"));
        assert_eq!(
            data.get("return_url").map(String::as_str),
            Some("https://bank.example/submit")
        );
        assert_eq!(data.get("token").map(String::as_str), Some("abc123"));
        assert_eq!(data.get("ref").map(String::as_str), Some("MT-1001"));
    }

    #[test]
    fn content_types() {
        assert_eq!(file_content_type(std::path::Path::new("a.JPG")), "image/jpeg");
        assert_eq!(file_content_type(std::path::Path::new("a.pdf")), "application/pdf");
        assert_eq!(
            file_content_type(std::path::Path::new("a.bin")),
            "application/octet-stream"
        );
    }
}
