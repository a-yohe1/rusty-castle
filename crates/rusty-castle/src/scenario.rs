//! Scenario capture support for compatibility replay tests.

use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Records HTTP interactions as JSON Lines for later scenario replay.
#[derive(Clone)]
pub struct ScenarioRecorder {
    file: Arc<Mutex<File>>,
}

impl ScenarioRecorder {
    /// Opens a capture file, replacing any previous contents.
    pub fn create(path: &Path) -> io::Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(path)?;
        Ok(Self {
            file: Arc::new(Mutex::new(file)),
        })
    }

    /// Appends one captured interaction to the scenario file.
    pub fn record(&self, interaction: &RecordedInteraction<'_>) -> io::Result<()> {
        let line = interaction.to_json_line();
        let mut file = self
            .file
            .lock()
            .map_err(|_| io::Error::other("scenario recorder lock poisoned"))?;
        file.write_all(line.as_bytes())?;
        file.write_all(b"\n")?;
        file.flush()
    }
}

impl std::fmt::Debug for ScenarioRecorder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScenarioRecorder").finish_non_exhaustive()
    }
}

/// One HTTP request/response pair captured from a device interaction.
#[derive(Clone, Debug)]
pub struct RecordedInteraction<'a> {
    /// Request method.
    pub method: &'a str,
    /// Request path and query.
    pub path: &'a str,
    /// Request headers.
    pub request_headers: &'a [(String, String)],
    /// UTF-8 request body, usually a SOAP envelope.
    pub request_body: &'a str,
    /// Response status code.
    pub response_status: u16,
    /// Response content type.
    pub response_content_type: &'a str,
    /// Response headers.
    pub response_headers: &'a [(String, String)],
    /// UTF-8 response body, when the response is replay-friendly text.
    pub response_body: Option<&'a str>,
    /// Number of response bytes intentionally omitted from the capture.
    pub omitted_response_body_bytes: usize,
}

impl RecordedInteraction<'_> {
    fn to_json_line(&self) -> String {
        let mut out = String::new();
        out.push('{');
        json_field(&mut out, "schema", "rusty-castle.scenario.v1");
        out.push(',');
        json_field(&mut out, "method", self.method);
        out.push(',');
        json_field(&mut out, "path", self.path);
        out.push(',');
        json_header_array(&mut out, "request_headers", self.request_headers);
        out.push(',');
        json_field(&mut out, "request_body", self.request_body);
        out.push(',');
        json_number_field(&mut out, "response_status", u64::from(self.response_status));
        out.push(',');
        json_field(
            &mut out,
            "response_content_type",
            self.response_content_type,
        );
        out.push(',');
        json_header_array(&mut out, "response_headers", self.response_headers);
        out.push(',');
        match self.response_body {
            Some(body) => json_field(&mut out, "response_body", body),
            None => out.push_str("\"response_body\":null"),
        }
        out.push(',');
        json_number_field(
            &mut out,
            "omitted_response_body_bytes",
            self.omitted_response_body_bytes as u64,
        );
        out.push('}');
        out
    }
}

fn json_field(out: &mut String, name: &str, value: &str) {
    json_string(out, name);
    out.push(':');
    json_string(out, value);
}

fn json_number_field(out: &mut String, name: &str, value: u64) {
    json_string(out, name);
    out.push(':');
    out.push_str(&value.to_string());
}

fn json_header_array(out: &mut String, name: &str, headers: &[(String, String)]) {
    json_string(out, name);
    out.push_str(":[");
    for (index, (header, value)) in headers.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('[');
        json_string(out, header);
        out.push(',');
        json_string(out, value);
        out.push(']');
    }
    out.push(']');
}

fn json_string(out: &mut String, value: &str) {
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            ch if ch.is_control() => {
                use std::fmt::Write as _;
                let _ = write!(out, "\\u{:04x}", ch as u32);
            }
            ch => out.push(ch),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_interaction_as_json_line() {
        let interaction = RecordedInteraction {
            method: "POST",
            path: "/ContentDirectory/control",
            request_headers: &[("soapaction".into(), "\"Browse\"".into())],
            request_body: "<Body>\n</Body>",
            response_status: 200,
            response_content_type: "text/xml; charset=\"utf-8\"",
            response_headers: &[("Content-Length".into(), "4".into())],
            response_body: Some("<ok/>"),
            omitted_response_body_bytes: 0,
        };

        let line = interaction.to_json_line();

        assert!(line.contains("\"schema\":\"rusty-castle.scenario.v1\""));
        assert!(line.contains("\"method\":\"POST\""));
        assert!(line.contains("[\"soapaction\",\"\\\"Browse\\\"\"]"));
        assert!(line.contains("\"request_body\":\"<Body>\\n</Body>\""));
        assert!(line.contains("\"response_content_type\":\"text/xml; charset=\\\"utf-8\\\"\""));
    }
}
