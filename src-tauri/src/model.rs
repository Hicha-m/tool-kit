use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TaskRequest {
    pub operation: String, pub inputs: Vec<String>, pub destination: String,
    pub format: String, pub pages: String, pub width: u32, pub quality: u8, pub track: u16,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String, pub operation: String, pub status: String,
    pub created: i64, pub output: Option<String>, pub error: Option<String>,
}
#[derive(Debug, Clone, Serialize)]
pub struct Engine { pub id: String, pub installed: bool, pub version: Option<String>, pub available: bool }
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all="camelCase")]
pub struct Catalog { pub schema_version: u32, pub artifacts: Vec<Artifact> }
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all="camelCase")]
pub struct Artifact {
    pub id: String, pub version: String, pub platform: String, pub url: String,
    pub sha256: String, pub size: u64, pub unpacked_limit: u64,
    pub executable: String, pub probe: Option<String>, pub license: String,
}
pub fn engine_for(operation: &str) -> Result<&'static str, String> {
    match operation {
        "pdf.merge"|"pdf.extract" => Ok("qpdf"),
        "audio.convert"|"video.convert"|"video.audio" => Ok("ffmpeg"),
        "image.convert"|"image.resize" => Ok("imagemagick"),
        _ => Err("Cet outil n’est pas disponible.".into()),
    }
}
pub fn pages_valid(pages: &str, count: u32) -> bool {
    !pages.is_empty() && pages.split(',').all(|part| {
        let bounds: Vec<_> = part.split('-').collect();
        if bounds.is_empty() || bounds.len()>2 { return false }
        let start = bounds[0].parse::<u32>().ok();
        let end = bounds.last().and_then(|s|s.parse::<u32>().ok());
        matches!((start,end),(Some(a),Some(b)) if a>0 && a<=b && b<=count)
    })
}
#[cfg(test)] mod tests {
    use super::*;
    #[test] fn page_ranges_reject_injection_and_bounds() {
        for p in ["", "0", "4", "3-1", "1-2-3", "1,", "--", "1;echo"] { assert!(!pages_valid(p,3),"{p}"); }
        for p in ["1", "1-3", "1,3", "1-2,3"] { assert!(pages_valid(p,3)); }
    }
    #[test] fn unknown_operations_are_rejected() { assert!(engine_for("shell.exec").is_err()); }
}
