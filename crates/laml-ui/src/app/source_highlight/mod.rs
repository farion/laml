//! Source-only highlighting: no compilation, networking or document mutation.
mod classify;
mod fragments;
mod lexer;
mod members;
mod palette;
#[cfg(test)]
mod regressions;
#[cfg(test)]
mod tests;
mod words;
use eframe::egui;
use palette::Role;
pub(super) fn install_fonts(fonts: &mut egui::FontDefinitions) {
    palette::install_fonts(fonts);
}

pub(super) fn highlight(text: &str, dark: bool, normal: egui::Color32) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    let mut context = classify::Context::new(text);
    for line in text.split_inclusive('\n') {
        let tokens = lexer::lex(line);
        let roles = context.line(&tokens);
        for (token, role) in tokens.iter().zip(roles) {
            job.append(token.text, 0.0, role.format(dark, normal));
        }
    }
    job
}
