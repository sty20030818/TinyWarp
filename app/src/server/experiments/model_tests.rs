use warpui::{App, Entity, SingletonEntity};

use super::{ServerExperiment, ServerExperiments};
use crate::{GlobalResourceHandles, GlobalResourceHandlesProvider};

/// A model for testing purposes only.
///
/// We use it to demonstrate how client-side
/// models can be mutated to reflect server
/// experiment state changes.
pub struct TestModel(pub usize);

impl Entity for TestModel {
    type Event = ();
}
impl SingletonEntity for TestModel {}

fn initialize_app(
    app: &mut App,
    sender: Option<std::sync::mpsc::SyncSender<crate::persistence::ModelEvent>>,
) {
    app.update(crate::settings::init_and_register_user_preferences);

    let mut global_resources = GlobalResourceHandles::mock(app);
    global_resources.model_event_sender = sender;
    app.add_singleton_model(|_| GlobalResourceHandlesProvider::new(global_resources));
}

#[test]
fn test_new_from_cached() {
    App::test((), |mut app| async move {
        initialize_app(&mut app, None);

        let model = app.add_singleton_model(|_| TestModel(0));
        let cache = vec![ServerExperiment::TestExperiment];
        app.add_singleton_model(|ctx| ServerExperiments::new_from_cache(cache, ctx));

        // The experiment should have been enabled.
        model.read(&app, |model, _| {
            assert_eq!(model.0, 1);
        });
    });
}

#[test]
fn test_apply_latest_state() {
    App::test((), |mut app| async move {
        initialize_app(&mut app, None);

        let model = app.add_singleton_model(|_| TestModel(0));
        let experiments =
            app.add_singleton_model(|ctx| ServerExperiments::new_from_cache(vec![], ctx));

        // Enable the experiment.
        experiments.update(&mut app, |experiments, ctx| {
            experiments.apply_latest_state(vec![ServerExperiment::TestExperiment], ctx);
        });
        model.read(&app, |model, _| {
            assert_eq!(model.0, 1);
        });

        // Redundant experiment state should be a no-op.
        experiments.update(&mut app, |experiments, ctx| {
            experiments.apply_latest_state(vec![ServerExperiment::TestExperiment], ctx);
        });
        model.read(&app, |model, _| {
            assert_eq!(model.0, 1);
        });
    });
}

#[test]
fn local_terminal_ignores_experiment_cache_and_server_updates_without_writes() {
    App::test((), |mut app| async move {
        let (sender, receiver) = std::sync::mpsc::sync_channel(16);
        initialize_app(&mut app, Some(sender));
        app.update(crate::local_terminal::enable_for_test);
        let counter = app.add_singleton_model(|_| TestModel(0));
        let experiments = app.add_singleton_model(|ctx| {
            ServerExperiments::new_from_cache(vec![ServerExperiment::TestExperiment], ctx)
        });
        experiments.update(&mut app, |state, ctx| {
            state.apply_latest_state(vec![ServerExperiment::TestExperiment], ctx);
            assert!(!state.is_experiment_enabled(&ServerExperiment::TestExperiment));
        });
        counter.read(&app, |counter, _| assert_eq!(counter.0, 0));
        assert!(matches!(
            receiver.try_recv(),
            Err(std::sync::mpsc::TryRecvError::Empty)
        ));
    });
}

#[test]
fn normal_experiment_updates_preserve_persistence_and_authoritative_empty_state() {
    App::test((), |mut app| async move {
        let (sender, receiver) = std::sync::mpsc::sync_channel(16);
        initialize_app(&mut app, Some(sender));
        let counter = app.add_singleton_model(|_| TestModel(0));
        let experiments = app.add_singleton_model(|ctx| {
            ServerExperiments::new_from_cache(vec![ServerExperiment::TestExperiment], ctx)
        });
        counter.read(&app, |counter, _| assert_eq!(counter.0, 1));
        assert!(
            matches!(receiver.try_recv(), Ok(crate::persistence::ModelEvent::SaveExperiments { experiments }) if experiments == vec![ServerExperiment::TestExperiment])
        );
        experiments.update(&mut app, |state, ctx| state.apply_latest_state(vec![], ctx));
        assert!(
            matches!(receiver.try_recv(), Ok(crate::persistence::ModelEvent::SaveExperiments { experiments }) if experiments.is_empty())
        );
    });
}
