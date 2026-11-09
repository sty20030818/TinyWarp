use std::cell::Cell;
use std::rc::Rc;

use ai::api_keys::CustomEndpointSchema;
use settings::Setting as _;
use warpui::{App, SingletonEntity as _};
use warpui_extras::secure_storage;

use super::*;
use crate::settings::init_and_register_user_preferences;

struct FailingSecureStorage {
    write_attempts: Rc<Cell<usize>>,
}

impl secure_storage::SecureStorage for FailingSecureStorage {
    fn write_value(&self, _key: &str, _value: &str) -> Result<(), secure_storage::Error> {
        self.write_attempts.set(self.write_attempts.get() + 1);
        Err(secure_storage::Error::Unknown(anyhow::anyhow!(
            "deliberate test failure"
        )))
    }

    fn read_value(&self, _key: &str) -> Result<String, secure_storage::Error> {
        Err(secure_storage::Error::NotFound)
    }

    fn remove_value(&self, _key: &str) -> Result<(), secure_storage::Error> {
        Ok(())
    }
}

#[test]
fn removal_succeeds_when_credential_cleanup_fails() {
    App::test((), |mut app| async move {
        app.update(init_and_register_user_preferences);
        app.add_singleton_model(AISettings::new_with_defaults);

        let write_attempts = Rc::new(Cell::new(0));
        let storage_write_attempts = write_attempts.clone();
        app.add_singleton_model(move |_| -> secure_storage::Model {
            Box::new(FailingSecureStorage {
                write_attempts: storage_write_attempts,
            })
        });
        app.add_singleton_model(ApiKeyManager::new);

        let mut definitions = CustomEndpointDefinitions::default();
        definitions
            .insert(
                CustomEndpointId::generated(),
                CustomEndpointDefinition {
                    name: "Test".to_owned(),
                    base_url: "https://example.com/v1".to_owned(),
                    schema: CustomEndpointSchema::default(),
                    models: vec![CustomEndpointModel {
                        name: "model".to_owned(),
                        alias: None,
                        config_key: "config-key".to_owned(),
                    }],
                },
            )
            .unwrap();
        AISettings::handle(&app)
            .update(&mut app, |settings, ctx| {
                settings
                    .custom_endpoints
                    .load_value(definitions.clone(), true, ctx)
            })
            .unwrap();
        ApiKeyManager::handle(&app).update(&mut app, |manager, ctx| {
            manager.set_custom_endpoint_definitions(definitions, ctx);
        });

        assert!(app.update(|ctx| remove(0, ctx)).is_ok());
        assert_eq!(write_attempts.get(), 1);
        AISettings::handle(&app).read(&app, |settings, _| {
            assert!(settings.custom_endpoints.value().is_empty());
        });
        ApiKeyManager::handle(&app).read(&app, |manager, _| {
            assert!(
                manager
                    .custom_endpoint_definitions()
                    .is_some_and(CustomEndpointDefinitions::is_empty)
            );
            assert!(manager.custom_endpoints().is_empty());
        });
    });
}

#[test]
fn local_terminal_custom_endpoint_model_stays_idle_without_cloud_dependencies() {
    App::test((), |mut app| async move {
        app.update(crate::local_terminal::enable_for_test);
        let model = app.add_singleton_model(|ctx| {
            CustomEndpointSettingsModel::new(&LaunchMode::new_for_unit_test(), ctx)
        });
        model.update(&mut app, |model, ctx| {
            assert!(!model.imports_legacy_endpoints);
            model.settings_invalid = true;
            model.sync_or_migrate(ctx);
            assert!(!ctx.has_singleton_model::<ApiKeyManager>());
            assert!(!ctx.has_singleton_model::<CloudPreferencesSyncer>());
            assert!(!ctx.has_singleton_model::<WarpConfig>());
        });
    });
}

struct EndpointStorageFixture {
    values: Rc<std::cell::RefCell<std::collections::HashMap<String, String>>>,
    writes: Rc<Cell<usize>>,
    removals: Rc<Cell<usize>>,
}

impl secure_storage::SecureStorage for EndpointStorageFixture {
    fn write_value(&self, key: &str, value: &str) -> Result<(), secure_storage::Error> {
        self.writes.set(self.writes.get() + 1);
        self.values
            .borrow_mut()
            .insert(key.to_owned(), value.to_owned());
        Ok(())
    }

    fn read_value(&self, key: &str) -> Result<String, secure_storage::Error> {
        self.values
            .borrow()
            .get(key)
            .cloned()
            .ok_or(secure_storage::Error::NotFound)
    }

    fn remove_value(&self, key: &str) -> Result<(), secure_storage::Error> {
        self.removals.set(self.removals.get() + 1);
        self.values.borrow_mut().remove(key);
        Ok(())
    }
}

#[test]
fn local_terminal_custom_endpoint_changes_preserve_settings_and_keys() {
    App::test((), |mut app| async move {
        app.update(init_and_register_user_preferences);
        app.add_singleton_model(AISettings::new_with_defaults);
        app.update(crate::local_terminal::enable_for_test);
        let legacy_endpoint = ai::api_keys::CustomEndpoint {
            name: "旧配置".to_owned(),
            url: "https://example.invalid/v1".to_owned(),
            api_key: "fixture-legacy-key".to_owned(),
            schema: Default::default(),
            models: vec![CustomEndpointModel {
                name: "fixture-model".to_owned(),
                alias: None,
                config_key: "fixture-model-id".to_owned(),
            }],
        };
        let legacy_keys = ai::api_keys::ApiKeys {
            custom_endpoints: vec![legacy_endpoint.clone()],
            ..Default::default()
        };
        let (definitions, keys) =
            CustomEndpointDefinitions::from_legacy(std::slice::from_ref(&legacy_endpoint)).unwrap();
        let id = definitions.id_at(0).unwrap().clone();
        let values = Rc::new(std::cell::RefCell::new(std::collections::HashMap::from([
            (
                "AiApiKeys".to_owned(),
                serde_json::to_string(&legacy_keys).unwrap(),
            ),
            (
                "AiCustomEndpointKeys".to_owned(),
                serde_json::to_string(&keys).unwrap(),
            ),
        ])));
        let original_storage = values.borrow().clone();
        let writes = Rc::new(Cell::new(0));
        let removals = Rc::new(Cell::new(0));
        app.add_singleton_model({
            let values = values.clone();
            let writes = writes.clone();
            let removals = removals.clone();
            move |_| -> secure_storage::Model {
                Box::new(EndpointStorageFixture {
                    values,
                    writes,
                    removals,
                })
            }
        });
        let manager = app.add_singleton_model(ApiKeyManager::new);
        manager.update(&mut app, |manager, ctx| {
            manager.set_custom_endpoint_definitions(definitions.clone(), ctx)
        });
        AISettings::handle(&app)
            .update(&mut app, |settings, ctx| {
                settings
                    .custom_endpoints
                    .load_value(definitions.clone(), true, ctx)
            })
            .unwrap();
        let model = app.add_singleton_model(|ctx| {
            CustomEndpointSettingsModel::new(&LaunchMode::new_for_unit_test(), ctx)
        });
        let params = || CustomEndpointParams {
            name: "新配置".to_owned(),
            url: "https://new.example.invalid/v1".to_owned(),
            api_key: "fixture-replacement-key".to_owned(),
            models: vec![],
            schema: Default::default(),
        };
        for error in [
            app.update(|ctx| add(params(), ctx).map(|_| ()))
                .unwrap_err(),
            app.update(|ctx| save(0, params(), ctx)).unwrap_err(),
            app.update(|ctx| remove(0, ctx)).unwrap_err(),
        ] {
            assert!(error.to_string().contains("unavailable"));
        }
        model.update(&mut app, |model, ctx| {
            model.imports_legacy_endpoints = true;
            model.settings_invalid = true;
            model.sync_or_migrate(ctx);
        });
        AISettings::handle(&app).read(&app, |settings, _| {
            assert_eq!(settings.custom_endpoints.value(), &definitions);
            assert!(settings.custom_endpoints.is_value_explicitly_set());
        });
        manager.read(&app, |manager, _| {
            assert_eq!(manager.keys().custom_endpoints, vec![legacy_endpoint]);
            assert_eq!(manager.custom_endpoint_definitions(), Some(&definitions));
            assert_eq!(manager.custom_endpoint_key(&id), Some("fixture-legacy-key"));
        });
        assert_eq!(writes.get(), 0);
        assert_eq!(removals.get(), 0);
        assert_eq!(*values.borrow(), original_storage);
    });
}
