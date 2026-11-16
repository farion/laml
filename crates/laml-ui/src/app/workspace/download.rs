//! Compact native-download chooser; platform detection stays on the client.
use super::*;
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Os {
    Windows,
    Mac,
    Linux,
    #[default]
    Unknown,
}
impl Os {
    fn label(self) -> &'static str {
        crate::i18n::tr(match self {
            Self::Windows => "Windows",
            Self::Mac => "macOS",
            Self::Linux => "Linux",
            Self::Unknown => "System not detected",
        })
    }
}
#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Arch {
    X64,
    Arm64,
    Unsupported,
    #[default]
    Unknown,
}
impl Arch {
    fn label(self) -> &'static str {
        crate::i18n::tr(match self {
            Self::X64 => "Intel / AMD 64-bit",
            Self::Arm64 => "ARM64 / Apple Silicon",
            Self::Unsupported => "Unsupported architecture",
            Self::Unknown => "architecture not detected",
        })
    }
}
#[derive(Clone, Copy, Default)]
struct Platform {
    os: Os,
    arch: Arch,
}
impl Platform {
    fn target(self) -> Option<(&'static str, &'static str)> {
        match (self.os, self.arch) {
            (Os::Windows, Arch::X64) => Some(("x86_64-pc-windows-msvc", "zip")),
            (Os::Mac, Arch::X64) => Some(("x86_64-apple-darwin", "tar.gz")),
            (Os::Mac, Arch::Arm64) => Some(("aarch64-apple-darwin", "tar.gz")),
            (Os::Linux, Arch::X64) => Some(("x86_64-unknown-linux-gnu", "tar.gz")),
            _ => None,
        }
    }
}

#[derive(Default)]
pub(super) struct State {
    pub(super) open: bool,
    started: bool,
    platform: Rc<RefCell<Platform>>,
}
impl State {
    pub(super) fn show(&mut self, ctx: &egui::Context) {
        self.open = true;
        if !self.started {
            self.started = true;
            detect(self.platform.clone(), ctx.clone());
        }
    }
}

impl Workspace {
    pub(super) fn download_window(&mut self, ctx: &egui::Context) {
        if !self.download.open {
            return;
        }
        let platform = *self.download.platform.borrow();
        let mut close = false;
        let dismissed = dialogs::show(
            ctx,
            "local-downloads",
            "Download LAML from github.com",
            410.0,
            |ui| {
                ui.spacing_mut().item_spacing.y = 8.0;
                let target = platform.target();
                let system = format!(
                    "v{} · {} · {}",
                    env!("CARGO_PKG_VERSION"),
                    platform.os.label(),
                    platform.arch.label(),
                );
                let button_width = ui.available_width();
                if download_button(
                    ui,
                    "LAML Graphical Editor",
                    &system,
                    true,
                    button_width,
                    target.is_some(),
                )
                .clicked()
                {
                    open_asset(ui, "laml-ui", target.unwrap());
                }
                if download_button(
                    ui,
                    "LAML CLI only",
                    &system,
                    false,
                    button_width,
                    target.is_some(),
                )
                .clicked()
                {
                    open_asset(ui, "laml", target.unwrap());
                }
                if target.is_none() {
                    ui.small(crate::i18n::tr(
                        "No matching download detected. Please use Other Releases.",
                    ));
                }
                ui.add_space(4.0);
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.link(crate::i18n::tr("Other Releases")).clicked() {
                        ui.ctx().open_url(egui::OpenUrl::new_tab(
                            "https://github.com/farion/laml/releases/latest",
                        ));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        close = ui.button(crate::i18n::tr("Close")).clicked();
                    });
                });
            },
        );
        if dismissed || close {
            self.download.open = false;
        }
    }
}

fn download_button(
    ui: &mut egui::Ui,
    title: &str,
    system: &str,
    primary: bool,
    width: f32,
    enabled: bool,
) -> egui::Response {
    ui.add_enabled_ui(enabled, |ui| {
        let mut button = egui::Button::new("");
        if primary {
            button = button
                .fill(ui.visuals().selection.bg_fill)
                .stroke(ui.visuals().selection.stroke);
        }
        let response = ui
            .add_sized([width, if primary { 54.0 } else { 44.0 }], button)
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        let center = response.rect.center();
        let color = ui.style().interact(&response).text_color();
        let painter = ui
            .painter()
            .with_clip_rect(response.rect.intersect(ui.clip_rect()));
        if primary && response.enabled() && response.hovered() {
            let tint = if ui.visuals().dark_mode {
                egui::Color32::from_white_alpha(24)
            } else {
                egui::Color32::from_black_alpha(18)
            };
            painter.rect_filled(response.rect.shrink(1.0), 4.0, tint);
        }
        painter.text(
            center - egui::vec2(0.0, 8.0),
            egui::Align2::CENTER_CENTER,
            crate::i18n::tr(title),
            egui::FontId::proportional(if primary { 17.0 } else { 14.0 }),
            color,
        );
        painter.text(
            center + egui::vec2(0.0, 10.0),
            egui::Align2::CENTER_CENTER,
            system,
            egui::FontId::proportional(11.0),
            color.gamma_multiply(0.75),
        );
        response
    })
    .inner
}

fn open_asset(ui: &egui::Ui, package: &str, (target, extension): (&str, &str)) {
    let version = env!("CARGO_PKG_VERSION");
    ui.ctx().open_url(egui::OpenUrl::new_tab(format!(
        "https://github.com/farion/laml/releases/download/v{version}/{package}-v{version}-{target}.{extension}"
    )));
}

#[cfg(not(target_arch = "wasm32"))]
fn detect(platform: Rc<RefCell<Platform>>, _ctx: egui::Context) {
    *platform.borrow_mut() = Platform {
        os: match std::env::consts::OS {
            "windows" => Os::Windows,
            "macos" => Os::Mac,
            "linux" => Os::Linux,
            _ => Os::Unknown,
        },
        arch: match std::env::consts::ARCH {
            "x86_64" => Arch::X64,
            "aarch64" => Arch::Arm64,
            _ => Arch::Unsupported,
        },
    };
}

#[cfg(target_arch = "wasm32")]
fn detect(platform: Rc<RefCell<Platform>>, ctx: egui::Context) {
    use wasm_bindgen::prelude::*;
    #[wasm_bindgen(inline_js = "
export async function lamlDownloadPlatform() {
    const ua = navigator.userAgent.toLowerCase();
    let os = /android|iphone|ipad|ipod/.test(ua) ? ''
        : ua.includes('windows') ? 'windows'
        : ua.includes('macintosh') ? 'macos'
        : ua.includes('linux') ? 'linux' : '';
    let arch = os === 'macos' ? ''
        : /aarch64|arm64/.test(ua) ? 'arm64'
        : /x86_64|x64|win64|wow64|amd64/.test(ua) ? 'x64'
        : /i[3-6]86|armv7/.test(ua) ? 'unsupported' : '';
    try {
        const data = await navigator.userAgentData?.getHighEntropyValues(['architecture', 'bitness']);
        if (data && os) {
            const platforms = {Windows:'windows', macOS:'macos', Linux:'linux'};
            os = platforms[data.platform] || os;
            if (data.architecture && data.bitness) {
                arch = data.bitness !== '64' ? 'unsupported'
                    : data.architecture === 'arm' ? 'arm64'
                    : data.architecture === 'x86' ? 'x64' : 'unsupported';
            }
        }
    } catch (_) {}
    return os + ':' + arch;
}")]
    extern "C" {
        #[wasm_bindgen(catch, js_name = lamlDownloadPlatform)]
        async fn browser_platform() -> Result<JsValue, JsValue>;
    }
    wasm_bindgen_futures::spawn_local(async move {
        if let Ok(value) = browser_platform().await {
            let value = value.as_string().unwrap_or_default();
            let (os, arch) = value.split_once(':').unwrap_or_default();
            *platform.borrow_mut() = Platform {
                os: match os {
                    "windows" => Os::Windows,
                    "macos" => Os::Mac,
                    "linux" => Os::Linux,
                    _ => Os::Unknown,
                },
                arch: match arch {
                    "x64" => Arch::X64,
                    "arm64" => Arch::Arm64,
                    "unsupported" => Arch::Unsupported,
                    _ => Arch::Unknown,
                },
            };
            ctx.request_repaint();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn released_targets_match_packages() {
        for (os, arch, target, extension) in [
            (Os::Windows, Arch::X64, "x86_64-pc-windows-msvc", "zip"),
            (Os::Mac, Arch::X64, "x86_64-apple-darwin", "tar.gz"),
            (Os::Mac, Arch::Arm64, "aarch64-apple-darwin", "tar.gz"),
            (Os::Linux, Arch::X64, "x86_64-unknown-linux-gnu", "tar.gz"),
        ] {
            assert_eq!(Platform { os, arch }.target(), Some((target, extension)));
        }
        for platform in [
            Platform::default(),
            Platform {
                os: Os::Mac,
                arch: Arch::Unknown,
            },
            Platform {
                os: Os::Linux,
                arch: Arch::Arm64,
            },
            Platform {
                os: Os::Windows,
                arch: Arch::Unsupported,
            },
        ] {
            assert!(platform.target().is_none());
        }
    }
}
