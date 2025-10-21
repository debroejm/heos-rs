use eframe::{CreationContext, Frame};
use egui::{Context, ViewportCommand};
use egui_async::{Bind, EguiAsyncPlugin};
use heos::HeosConnection;
use std::sync::Arc;
use std::time::Duration;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

use crate::screen::loaded::Loaded;
use crate::screen::media_bar::MediaBar;

mod screen;
mod assets;
mod util;

fn main() -> eframe::Result {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .finish()
        .init();

    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "HEOS Control",
        options,
        Box::new(|ctx| Ok(Box::new(HeosControlApp::new(ctx)))),
    )
}

struct HeosControlApp {
    inner: Bind<Loaded, String>,
}

impl HeosControlApp {
    async fn init() -> Result<Loaded, String> {
        let adhoc = match HeosConnection::connect_any(Duration::from_secs(10)).await {
            Ok(adhoc) => adhoc,
            Err(error) => return Err(format!("Failed to connect to any HEOS devices: {error}")),
        };

        let heos = match adhoc.init_stateful().await {
            Ok(heos) => heos,
            Err(error) => return Err(format!("Failed to initialize HEOS system state: {error}")),
        };

        Ok(Loaded::new(Arc::new(heos)))
    }

    fn new(ctx: &CreationContext) -> Self {
        egui_extras::install_image_loaders(&ctx.egui_ctx);

        // TODO: This might need to move
        ctx.egui_ctx.send_viewport_cmd(ViewportCommand::MinInnerSize(MediaBar::MIN_SIZE));

        Self {
            inner: Bind::new(true),
        }
    }
}

impl eframe::App for HeosControlApp {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        ctx.plugin_or_default::<EguiAsyncPlugin>();

        match self.inner.read_mut_or_request(Self::init) {
            Some(result) => match result {
                Ok(loaded) => {
                    loaded.update(ctx);
                },
                Err(msg) => {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::TopDown), |ui| {
                            ui.heading(msg);
                        });
                    });
                }
            },
            None => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::TopDown), |ui| {
                        ui.add(egui::widgets::Spinner::new().size(100.0));
                    });
                });
            }
        }
    }
}