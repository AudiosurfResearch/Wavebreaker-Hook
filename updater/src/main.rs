#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::{borrow::Cow, io::Cursor, path::Path, time::Duration};

use anyhow::{anyhow, bail, Context};
use eframe::egui::{
    self, Align, FontData, FontDefinitions, FontFamily, Layout, ProgressBar, RichText, Vec2,
    ViewportBuilder, ViewportCommand,
};
use lazy_async_promise::{
    ImmediateValuePromise, ImmediateValueState, Progress, ProgressTrackedImValProm, StringStatus,
};
use sysinfo::{ProcessRefreshKind, RefreshKind, System};

#[tokio::main]
async fn main() -> eframe::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("wavebreaker_up=debug")
        .init();

    let viewport_options = ViewportBuilder {
        title: Some("Wavebreaker Updater".to_owned()),
        inner_size: Some(Vec2::new(650.0, 100.0)),
        resizable: Some(false),
        maximize_button: Some(false),
        icon: None,
        ..Default::default()
    };

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
    update_task: ProgressTrackedImValProm<(), Cow<'static, str>>,
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

        Self {
            update_task: Self::run_update(),
        }
    }

    fn run_update() -> ProgressTrackedImValProm<(), Cow<'static, str>> {
        ProgressTrackedImValProm::new(
            |s| {
                ImmediateValuePromise::new(async move {
                    let mut system = System::new_with_specifics(
                        RefreshKind::new().with_processes(ProcessRefreshKind::everything()),
                    );

                    // Start by waiting for the game to close
                    s.send(StringStatus::new(
                        Progress::from_percent(0),
                        "Waiting for the game to close".into(),
                    ))
                    .await
                    .unwrap();
                    while !system
                        .processes_by_exact_name("QuestViewer.exe")
                        .collect::<Vec<_>>()
                        .is_empty()
                    {
                        tokio::time::sleep(Duration::from_millis(500)).await;
                        system.refresh_processes_specifics(
                            ProcessRefreshKind::new().with_exe(sysinfo::UpdateKind::OnlyIfNotSet),
                        );
                    }

                    s.send(StringStatus::new(
                        Progress::from_percent(20),
                        "Getting release from GitHub".into(),
                    ))
                    .await
                    .unwrap();
                    let octocrab = octocrab::instance();
                    let repo = octocrab.repos("AudiosurfResearch", "Wavebreaker-Hook");
                    let release = repo
                        .releases()
                        .get_latest()
                        .await
                        .context("Failed to get releases from repo")?;
                    let release_asset_url = release
                        .assets
                        .iter()
                        .find(|asset| asset.name == "Wavebreaker-Package.zip")
                        .map(|asset| asset.browser_download_url.clone())
                        .ok_or_else(|| anyhow!("Failed to find asset in latest release"))?;

                    s.send(StringStatus::new(
                        Progress::from_percent(40),
                        format!("Get: {}", release_asset_url.path()).into(),
                    ))
                    .await
                    .unwrap();
                    let response = reqwest::get(release_asset_url)
                        .await
                        .context("Failed to download release asset")?;
                    let bytes = response
                        .bytes()
                        .await
                        .context("Failed to get response bytes")?;

                    s.send(StringStatus::new(
                        Progress::from_percent(60),
                        "Extracting files".into(),
                    ))
                    .await
                    .unwrap();
                    if !Path::new("./channels").exists() && !Path::new("./3rd").exists() {
                        bail!("Invalid folder structure! Is this really the game's engine folder?");
                    }
                    // automatically overwrites existing files
                    zip_extract::extract(Cursor::new(bytes), Path::new("."), false)
                        .context("Failed to extract zip")?;

                    s.send(StringStatus::new(
                        Progress::from_percent(80),
                        "Launching game".into(),
                    ))
                    .await
                    .unwrap();
                    open::that_detached("steam://launch/12900")
                        .context("Failed to launch game!")?;

                    // Wait until the game is launched
                    while system
                        .processes_by_exact_name("QuestViewer.exe")
                        .collect::<Vec<_>>()
                        .is_empty()
                    {
                        tokio::time::sleep(Duration::from_millis(500)).await;
                        system.refresh_processes_specifics(
                            ProcessRefreshKind::new().with_exe(sysinfo::UpdateKind::OnlyIfNotSet),
                        );
                    }

                    s.send(StringStatus::new(
                        Progress::from_percent(100),
                        "Done!".into(),
                    ))
                    .await
                    .unwrap();

                    Ok(())
                })
            },
            2000,
        )
    }
}

impl eframe::App for MyEguiApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.with_layout(
                Layout::top_down(Align::Center).with_cross_align(Align::Min),
                |ui| {
                    ui.heading("Updating");
                    ui.label("Keep the game closed, wait for the update to finish and don't exit the updater.");
                    ui.add_space(10.0);
                    let state = self.update_task.poll_state();
                    match state {
                        ImmediateValueState::Updating => {
                            if let Some(status) = self.update_task.last_status() {
                                ui.add(
                                    ProgressBar::new(status.progress.as_f32()).desired_height(12.0),
                                );
                                ui.label(RichText::new(status.message.clone()));
                                ctx.request_repaint(); // constantly requests UI to be redrawn, so the progress bar updates without user interaction
                            } else {
                                ui.label("Preparing for update...");
                            }
                        }
                        ImmediateValueState::Success(_) => {
                            ui.label(
                                RichText::new("Done!").color(catppuccin_egui::MACCHIATO.green),
                            );
                            ctx.send_viewport_cmd(ViewportCommand::Close)
                        }
                        ImmediateValueState::Error(err) => {
                            ui.label(
                                RichText::new(err.to_string())
                                    .color(catppuccin_egui::MACCHIATO.red),
                            );
                        }
                        _ => {}
                    }
                },
            );
        });
    }
}
