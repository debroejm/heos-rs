use async_trait::async_trait;
use egui_async::bind::MaybeSend;
use egui_async::Bind;
use futures_lite::FutureExt;
use heos::data::event::Event;
use heos::{HeosConnection, Stateful};
use parking_lot::Mutex;
use std::convert::Infallible;
use std::sync::{Arc, Weak};
use tokio::sync::Mutex as AsyncMutex;

struct UpdateEntryTyped<T, E, EventCheck, EventCheckFut, UpdateFn, UpdateFnFut>
where
    T: MaybeSend + 'static,
    E: MaybeSend + 'static,
    EventCheck: Fn(Event) -> EventCheckFut + Send + Sync + 'static,
    EventCheckFut: Future<Output = bool> + MaybeSend + 'static,
    UpdateFn: Fn() -> UpdateFnFut + Send + Sync + 'static,
    UpdateFnFut: Future<Output = Result<T, E>> + MaybeSend + 'static,
{
    data_bind: Weak<Mutex<Bind<T, E>>>,
    event_check: EventCheck,
    update_fn: UpdateFn,
    queued: bool,
}

impl<T, E, EventCheck, EventCheckFut, UpdateFn, UpdateFnFut> UpdateEntryTyped<T, E, EventCheck, EventCheckFut, UpdateFn, UpdateFnFut>
where
    T: MaybeSend + 'static,
    E: MaybeSend + 'static,
    EventCheck: Fn(Event) -> EventCheckFut + Send + Sync + 'static,
    EventCheckFut: Future<Output = bool> + MaybeSend + 'static,
    UpdateFn: Fn() -> UpdateFnFut + Send + Sync + 'static,
    UpdateFnFut: Future<Output = Result<T, E>> + MaybeSend + 'static,
{
    fn new(data_bind: &Arc<Mutex<Bind<T, E>>>, event_check: EventCheck, update_fn: UpdateFn) -> Self {
        Self {
            data_bind: Arc::downgrade(data_bind),
            event_check,
            update_fn,
            queued: false,
        }
    }
}

#[async_trait]
trait UpdateEntry: Send + Sync {
    fn is_active(&self) -> bool;
    async fn check_update(&mut self, event: Event);
    fn check_queued(&mut self);
}

#[async_trait]
impl<T, E, EventCheck, EventCheckFut, UpdateFn, UpdateFnFut> UpdateEntry for UpdateEntryTyped<T, E, EventCheck, EventCheckFut, UpdateFn, UpdateFnFut>
where
    T: MaybeSend + 'static,
    E: MaybeSend + 'static,
    EventCheck: Fn(Event) -> EventCheckFut + Send + Sync + 'static,
    EventCheckFut: Future<Output = bool> + MaybeSend + 'static,
    UpdateFn: Fn() -> UpdateFnFut + Send + Sync + 'static,
    UpdateFnFut: Future<Output = Result<T, E>> + MaybeSend + 'static,
{
    fn is_active(&self) -> bool {
        self.data_bind.strong_count() > 0
    }

    async fn check_update(&mut self, event: Event) {
        if (self.event_check)(event).await {
            if let Some(data_bind) = self.data_bind.upgrade() {
                let mut data_bind = data_bind.lock();
                if data_bind.is_pending() {
                    self.queued = true;
                } else {
                    self.queued = false;
                    data_bind.request((self.update_fn)());
                }
            }
        }
    }

    fn check_queued(&mut self) {
        if self.queued {
            if let Some(data_bind) = self.data_bind.upgrade() {
                let mut data_bind = data_bind.lock();
                if !data_bind.is_pending() {
                    data_bind.request((self.update_fn)());
                }
            } else {
                // Prevent any more spurious attempts, since the data_bind is already gone
                self.queued = false;
            }
        }
    }
}

pub struct Updater {
    bind: Bind<(), Infallible>,
    exit: Option<tokio::sync::oneshot::Sender<()>>,
    entries: Arc<AsyncMutex<Vec<Box<dyn UpdateEntry>>>>,
}

impl Updater {
    pub fn new(heos: Arc<HeosConnection<Stateful>>) -> Self {
        let (exit, exit_check) = tokio::sync::oneshot::channel();
        let mut bind = Bind::new(true);
        let entries: Arc<AsyncMutex<Vec<Box<dyn UpdateEntry>>>> = Arc::new(AsyncMutex::new(vec![]));

        {
            let entries = entries.clone();
            bind.request(async move {
                let mut event_recv = heos.subscribe_event_broadcast().await;
                let fut_loop = async { loop {
                    match event_recv.recv().await {
                        Ok(event) => {
                            let mut entries = entries.lock().await;
                            entries.retain(|entry| entry.is_active());
                            for entry in &mut *entries {
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
        let entry = UpdateEntryTyped::new(data_bind, event_check, update_fn);
        self.entries.blocking_lock().push(Box::new(entry));
    }

    pub fn check_queued(&self) {
        let mut entries = self.entries.blocking_lock();
        for entry in &mut *entries {
            entry.check_queued();
        }
    }

    pub fn into_bind(mut self) -> Bind<(), Infallible> {
        if let Some(exit) = self.exit.take() {
            let _ = exit.send(());
        }
        self.bind
    }
}