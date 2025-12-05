use std::convert::Infallible;
use eframe::{CreationContext, Frame};
use egui::{Context, RichText, ViewportCommand};
use egui_async::{Bind, EguiAsyncPlugin};
use heos::{HeosConnection, Stateful};
use std::sync::Arc;
use std::time::Duration;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

use crate::screen::loaded::Loaded;
use crate::screen::media_bar::MediaBar;
use crate::updater::Updater;

mod screen;
mod assets;
mod updater;
mod widgets;

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

enum State {
    Init(Bind<Arc<HeosConnection<Stateful>>, String>),
    Active {
        loaded: Loaded,
        updater: Updater,
    },
    Errored(String),
    Exiting {
        update_bind: Bind<(), Infallible>,
    },
    Uninitialized,
}

struct HeosControlApp {
    state: State,
}

impl HeosControlApp {
    async fn init_heos() -> Result<Arc<HeosConnection<Stateful>>, String> {
        let adhoc = match HeosConnection::connect_any(Duration::from_secs(10)).await {
            Ok(adhoc) => adhoc,
            Err(error) => return Err(format!("Failed to connect to any HEOS devices: {error}")),
        };

        let heos = match adhoc.init_stateful().await {
            Ok(heos) => heos,
            Err(error) => return Err(format!("Failed to initialize HEOS system state: {error}")),
        };

        Ok(Arc::new(heos))
    }

    fn new(ctx: &CreationContext) -> Self {
        egui_extras::install_image_loaders(&ctx.egui_ctx);

        // TODO: This might need to move
        ctx.egui_ctx.send_viewport_cmd(ViewportCommand::MinInnerSize(MediaBar::MIN_SIZE));

        let mut bind = Bind::new(true);
        bind.request(Self::init_heos());

        Self {
            state: State::Init(bind),
        }
    }

    fn show_spinner(ctx: &Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::TopDown), |ui| {
                ui.add(egui::widgets::Spinner::new().size(100.0));
            });
        });
    }

    fn show_error(ctx: &Context, msg: impl Into<RichText>) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::TopDown), |ui| {
                ui.heading(msg);
            });
        });
    }
}

impl eframe::App for HeosControlApp {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        ctx.plugin_or_default::<EguiAsyncPlugin>();

        let mut state = State::Uninitialized;
        std::mem::swap(&mut self.state, &mut state);
        self.state = match state {
            State::Init(mut bind) => {
                match bind.take() {
                    Some(result) => match result {
                        Ok(heos) => {
                            Self::show_spinner(ctx);
                            ctx.request_repaint();

                            let updater = Updater::new(heos.clone());
                            let loaded = Loaded::new(heos, &updater);

                            State::Active {
                                loaded,
                                updater,
                            }
                        },
                        Err(msg) => {
                            Self::show_error(ctx, &msg);
                            ctx.request_repaint();
                            State::Errored(msg)
                        },
                    },
                    None => {
                        Self::show_spinner(ctx);
                        State::Init(bind)
                    },
                }
            },
            State::Active { mut loaded, updater } => {
                loaded.update(ctx, &updater);

                if ctx.input(|i| i.viewport().close_requested()) {
                    ctx.send_viewport_cmd(ViewportCommand::CancelClose);
                    let update_bind = updater.into_bind();
                    State::Exiting {
                        update_bind,
                    }
                } else {
                    State::Active {
                        loaded,
                        updater,
                    }
                }
            },
            State::Errored(msg) => {
                Self::show_error(ctx, &msg);
                State::Errored(msg)
            },
            State::Exiting { mut update_bind } => {
                if update_bind.is_finished() {
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                }
                State::Exiting {
                    update_bind,
                }
            },
            State::Uninitialized => unreachable!("Invalid state"),
        }
    }
}