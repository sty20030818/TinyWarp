use warpui::{Entity, ModelContext, SingletonEntity};

use crate::local_terminal;

#[derive(Clone)]
pub struct TeamTesterStatus {}

impl TeamTesterStatus {
    pub fn new(_ctx: &mut ModelContext<Self>) -> Self {
        Self {}
    }

    #[cfg(test)]
    pub fn mock(ctx: &mut ModelContext<Self>) -> Self {
        Self::new(ctx)
    }

    /// Emit an event to start or force-refresh the cloud object and workspace metadata pollers.
    /// Polling is started when a user logs in; this method is also called with
    /// `force_refresh: true` when data is known to be invalidated (e.g. joining a team via an
    /// intent link).
    pub fn initiate_data_pollers(&mut self, force_refresh: bool, ctx: &mut ModelContext<Self>) {
        if local_terminal::is_enabled(ctx) {
            return;
        }
        ctx.emit(TeamTesterStatusEvent::InitiateDataPollers { force_refresh })
    }
}

pub enum TeamTesterStatusEvent {
    InitiateDataPollers {
        /// If true, the subscriber should attempt to refresh any state
        /// immediately rather than just wait for the next poll.
        /// Specifically used when a user joins a team via an intent link.
        force_refresh: bool,
    },
}

impl Entity for TeamTesterStatus {
    type Event = TeamTesterStatusEvent;
}

impl SingletonEntity for TeamTesterStatus {}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use warpui::App;

    use super::*;

    #[test]
    fn local_terminal_does_not_emit_data_poller_start_events() {
        App::test((), |mut app| async move {
            app.update(crate::local_terminal::enable_for_test);
            let tester = app.add_singleton_model(TeamTesterStatus::new);
            let event_count = Arc::new(AtomicUsize::new(0));
            let observed_count = event_count.clone();
            app.update(|ctx| {
                ctx.subscribe_to_model(&tester, move |_, _, _| {
                    observed_count.fetch_add(1, Ordering::SeqCst);
                });
            });

            tester.update(&mut app, |tester, ctx| {
                tester.initiate_data_pollers(false, ctx);
                tester.initiate_data_pollers(true, ctx);
            });
            assert_eq!(event_count.load(Ordering::SeqCst), 0);
        });
    }
}
