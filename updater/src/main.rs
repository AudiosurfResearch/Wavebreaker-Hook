use std::sync::{Arc, Once};

use anyhow::Context;
use eframe::egui::{
    self, Align, FontData, FontDefinitions, FontFamily, Layout, ProgressBar, RichText, Vec2,
    ViewportBuilder,
};
use octocrab::{models::repos::Release, Octocrab};
use pollster::FutureExt as _;
use tracing::debug;

#[tokio::main]
async fn main() -> eframe::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("wavebreaker_up=debug")
        .init();

    let viewport_options = ViewportBuilder::default()
        .with_title("Wavebreaker Updater")
        .with_resizable(false)
        .with_inner_size(Vec2::new(650.0, 100.0))
        .with_maximize_button(false);

    let native_options = eframe::NativeOptions {
        viewport: viewport_options,
        centered: true,
        ..Default::default()
    };

    eframe::run_native(
        "WavebreakerUpdater",
        native_options,
        Box::new(|cc| Box::new(MyEguiApp::new(cc))),
    )
}

struct MyEguiApp {
    octocrab: Arc<Octocrab>,
    current_release: anyhow::Result<Release>,
    progress: f32,
}

impl MyEguiApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.
        let mut fonts = FontDefinitions::default();

        // Install Inter
        fonts.font_data.insert(
            "inter".to_owned(),
            FontData::from_static(include_bytes!("../fonts/Inter.ttf")),
        ); // .ttf and .otf supported

        // Give Inter the highest priority
        fonts
            .families
            .get_mut(&FontFamily::Proportional)
            .unwrap()
            .insert(0, "inter".to_owned());

        // Use Inter as fallback for monospace
        fonts
            .families
            .get_mut(&FontFamily::Monospace)
            .unwrap()
            .push("inter".to_owned());

        cc.egui_ctx.set_fonts(fonts);

        catppuccin_egui::set_theme(&cc.egui_ctx, catppuccin_egui::MACCHIATO);

        let octocrab = octocrab::instance();
        let repo = octocrab.repos("AudiosurfResearch", "Wavebreaker-Hook");

        Self {
            octocrab: octocrab::instance(),
            current_release: repo
                .releases()
                .get_latest()
                .block_on()
                .context("Failed to get latest release from repo"),
            progress: 0.0,
        }
    }
}

impl eframe::App for MyEguiApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.with_layout(
                Layout::top_down(Align::Center).with_cross_align(Align::Min),
                |ui| {
                    ui.heading("Updating");
                    ui.label("yuor'e computer will explosion");
                    ui.add_space(10.0);
                    ui.add(ProgressBar::new(self.progress).desired_height(12.0));
                    match &self.current_release {
                        Ok(release) => {
                            ui.label(format!("Downloading {} from GitHub", release.tag_name));
                            //TODO: how do i make things only run once? how do i handle state? what??
                            debug!("Downloading from {}", asset.browser_download_url.to_string());
                        }
                        Err(err) => {
                            ui.label(
                                RichText::new(err.to_string())
                                    .color(catppuccin_egui::MACCHIATO.red),
                            );
                        }
                    }
                },
            );
        });
    }
}
