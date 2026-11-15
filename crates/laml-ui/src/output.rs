//! Real compiler exports and checked, atomic local output writes.
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug)]
pub enum Format {
    Svg,
    Png,
    Pdf,
}
impl Format {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Svg => "svg",
            Self::Png => "png",
            Self::Pdf => "pdf",
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
pub use native::*;
