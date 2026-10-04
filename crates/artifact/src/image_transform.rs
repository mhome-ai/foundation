use serde::{Deserialize, Serialize};

/// Explicit image conversion. Only the converted bytes become an artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImageTransform {
    pub format: ImageOutputFormat,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_width: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_height: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quality: Option<u8>,
    /// Explicit RGB background for transparent input converted to JPEG.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ImageOutputFormat {
    Jpeg,
    Png,
}

impl ImageTransform {
    pub fn validate(&self) -> Result<(), &'static str> {
        for size in [self.max_width, self.max_height].into_iter().flatten() {
            if !(1..=16_384).contains(&size) {
                return Err("image transform dimensions must be between 1 and 16384");
            }
        }
        if let Some(quality) = self.quality {
            if self.format != ImageOutputFormat::Jpeg || !(1..=100).contains(&quality) {
                return Err("image transform quality requires JPEG and a value from 1 to 100");
            }
        }
        if let Some(color) = &self.background {
            if self.format != ImageOutputFormat::Jpeg
                || color.len() != 7
                || !color.starts_with('#')
                || !color.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit)
            {
                return Err("image transform background requires JPEG and #RRGGBB");
            }
        }
        Ok(())
    }
}
