use warpui::{App, SingletonEntity};

use super::InputClassifierModel;

#[test]
fn local_terminal_does_not_create_a_classifier() {
    App::test((), |mut app| async move {
        app.update(crate::local_terminal::enable_for_test);
        app.update(|ctx| {
            ctx.add_singleton_model(InputClassifierModel::new);
            assert!(InputClassifierModel::as_ref(ctx).classifier().is_none());
        });
    });
}

#[test]
fn nonlocal_classifier_follows_build_capability() {
    App::test((), |mut app| async move {
        app.update(|ctx| {
            ctx.add_singleton_model(InputClassifierModel::new);
            assert_eq!(
                InputClassifierModel::as_ref(ctx).classifier().is_some(),
                cfg!(feature = "input_classification")
            );
        });
    });
}
