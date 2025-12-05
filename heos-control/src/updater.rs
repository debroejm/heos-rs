use egui_async::bind::MaybeSend;
use egui_async::Bind;
use futures_lite::future::Boxed as BoxedFuture;
use futures_lite::FutureExt;
use heos::data::event::Event;
use heos::{HeosConnection, Stateful};
use parking_lot::Mutex;
use std::convert::Infallible;
use std::sync::Arc;

struct UpdateEntry {
    check_update_fn: Box<dyn (Fn(Event) -> BoxedFuture<()>) + Send + Sync + 'static>,
    is_active_fn: Box<dyn (Fn() -> bool) + Send + Sync + 'static>,
}

impl UpdateEntry {
    async fn check_update(&self, event: Event) -> () {
        (self.check_update_fn)(event).await
    }

    fn is_active(&self) -> bool {
        (self.is_active_fn)()
    }
}

pub struct Updater {
    bind: Bind<(), Infallible>,
    exit: Option<tokio::sync::oneshot::Sender<()>>,
    entries: Arc<Mutex<Vec<Arc<UpdateEntry>>>>,
}

impl Updater {
    pub fn new(heos: Arc<HeosConnection<Stateful>>) -> Self {
        let (exit, exit_check) = tokio::sync::oneshot::channel();
        let mut bind = Bind::new(true);
        let entries: Arc<Mutex<Vec<Arc<UpdateEntry>>>> = Arc::new(Mutex::new(vec![]));

        {
            let entries = entries.clone();
            bind.request(async move {
                let mut event_recv = heos.subscribe_event_broadcast().await;
                let fut_loop = async { loop {
                    match event_recv.recv().await {
                        Ok(event) => {
                            let entries = {
                                let mut entries = entries.lock();
                                entries.retain(|entry| entry.is_active());
                                entries.clone()
                            };
                            for entry in entries {
                                entry.check_update(event.clone()).await;
                            }
                        },
                        Err(_) => break,
                    }
                }};
                async {
                    let _ = exit_check.await;
                }.or(fut_loop).await;
                Ok::<(), Infallible>(())
            });
        }

        Self {
            bind,
            exit: Some(exit),
            entries,
        }
    }

    pub fn register<T, E, EventCheck, EventCheckFut, UpdateFn, UpdateFnFut>(
        &self,
        data_bind: &Arc<Mutex<Bind<T, E>>>,
        event_check: EventCheck,
        update_fn: UpdateFn,
    )
    where
        T: MaybeSend + 'static,
        E: MaybeSend + 'static,
        EventCheck: Fn(Event) -> EventCheckFut + Send + Sync + 'static,
        EventCheckFut: Future<Output = bool> + MaybeSend + 'static,
        UpdateFn: Fn() -> UpdateFnFut + Send + Sync + 'static,
        UpdateFnFut: Future<Output = Result<T, E>> + MaybeSend + 'static,
    {
        let data_bind = Arc::downgrade(data_bind);
        let check_update_fn = Box::new({
            let data_bind = data_bind.clone();
            let event_check = Arc::new(event_check);
            let update_fn = Arc::new(update_fn);
            move |event| {
                let data_bind = data_bind.clone();
                let event_check = event_check.clone();
                let update_fn = update_fn.clone();
                async move {
                    if event_check(event).await {
                        if let Some(data_bind) = data_bind.upgrade() {
                            data_bind.lock().request(update_fn());
                        }
                    }
                }.boxed()
            }
        });
        let is_active_fn = Box::new(move || data_bind.strong_count() > 0);
        let entry = Arc::new(UpdateEntry {
            check_update_fn,
            is_active_fn,
        });
        self.entries.lock().push(entry);
    }

    pub fn into_bind(mut self) -> Bind<(), Infallible> {
        if let Some(exit) = self.exit.take() {
            let _ = exit.send(());
        }
        self.bind
    }
}