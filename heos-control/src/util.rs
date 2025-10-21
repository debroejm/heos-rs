use egui::Color32;
use egui_async::Bind;
use futures_lite::FutureExt;
use heos::data::event::Event;
use heos::{HeosConnection, Stateful};
use std::convert::Infallible;
use std::sync::Arc;

pub fn normalized_gamma_multiply(original: Color32, mut factor: f32) -> Color32 {
    if original.intensity() < 0.5 {
        // Assume "dark mode", and flip the factor
        factor = 1.0 / factor;
        if factor < 1.0 {
            factor *= 0.875;
        } else {
            factor *= 1.25;
        }
    }
    original.gamma_multiply(factor)
}

pub struct Updater {
    inner: Bind<(), Infallible>,
    exit: Option<tokio::sync::oneshot::Sender<()>>,
}

impl Updater {
    pub fn new<F, Fut>(heos: Arc<HeosConnection<Stateful>>, f: F) -> Self
    where
        F: Fn(Event) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = bool> + Send + Sync + 'static,
    {
        let (exit, exit_check) = tokio::sync::oneshot::channel();
        let mut bind = Bind::new(true);
        bind.request(async move {
            let mut event_recv = heos.subscribe_event_broadcast().await;
            let fut_loop = async {
                loop {
                    match event_recv.recv().await {
                        Ok(event) => {
                            if f(event).await {
                                break
                            }
                        },
                        Err(_) => break,
                    }
                }
            };
            async {
                let _ = exit_check.await;
            }.or(fut_loop).await;
            Ok::<(), Infallible>(())
        });
        Self {
            inner: bind,
            exit: Some(exit),
        }
    }

    pub fn stop(&mut self) {
        // TODO: This will still result in a warn! trace because the receiver drops before the loop
        //  actually exits and returns. Is it worth it to add some sort of blocking wait? And how?
        if let Some(exit) = self.exit.take() {
            let _ = exit.send(());
            self.inner.clear();
        }
    }
}