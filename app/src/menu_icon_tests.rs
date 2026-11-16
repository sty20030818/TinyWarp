use image::{DynamicImage, GenericImageView as _, Rgb, RgbImage};
use tempfile::TempDir;
use warpui::App;

use super::*;

fn fixture(directory: &Path, name: &str, format: ImageFormat, color: [u8; 3]) -> PathBuf {
    let path = directory.join(name);
    let image = DynamicImage::ImageRgb8(RgbImage::from_pixel(300, 180, Rgb(color)));
    let mut bytes = Cursor::new(Vec::new());
    image.write_to(&mut bytes, format).unwrap();
    fs::write(&path, bytes.into_inner()).unwrap();
    path
}

#[test]
fn import_normalizes_png_and_jpeg_and_survives_original_removal() {
    let directory = TempDir::new().unwrap();
    let target = directory.path().join("menu-icons/custom.png");
    for (name, format) in [
        ("input.png", ImageFormat::Png),
        ("input.jpg", ImageFormat::Jpeg),
    ] {
        let source = fixture(directory.path(), name, format, [30, 140, 230]);
        let imported = import_image(&source, &target).unwrap();
        let bytes = fs::read(&target).unwrap();
        assert_eq!(image::guess_format(&bytes).unwrap(), ImageFormat::Png);
        assert_eq!(
            image::load_from_memory(&bytes).unwrap().dimensions(),
            (128, 128)
        );
        fs::remove_file(source).unwrap();
        let restarted = MenuIconManager::from_path(target.clone());
        assert_eq!(restarted.image_source(), Some(imported));
        assert!(restarted.last_error().is_none());
    }
}

#[test]
fn replacement_versions_the_actual_owned_file() {
    let directory = TempDir::new().unwrap();
    let target = directory.path().join("menu-icons/custom.png");
    let source = fixture(
        directory.path(),
        "input.png",
        ImageFormat::Png,
        [255, 90, 20],
    );
    let old = import_image(&source, &target).unwrap();
    fixture(
        directory.path(),
        "input.png",
        ImageFormat::Png,
        [20, 90, 255],
    );
    let new = import_image(&source, &target).unwrap();
    assert_ne!(old, new, "替换后不能复用旧图片缓存键");
    assert_eq!(load_stored_image(&target).unwrap(), Some(new));
}

#[test]
fn invalid_and_oversized_imports_preserve_the_previous_png() {
    let directory = TempDir::new().unwrap();
    let target = directory.path().join("menu-icons/custom.png");
    let source = fixture(
        directory.path(),
        "input.png",
        ImageFormat::Png,
        [20, 40, 60],
    );
    import_image(&source, &target).unwrap();
    let previous = fs::read(&target).unwrap();

    fs::write(&source, b"invalid image").unwrap();
    assert!(matches!(
        import_image(&source, &target),
        Err(MenuIconError::InvalidImage(_))
    ));
    assert_eq!(fs::read(&target).unwrap(), previous);

    let oversized = File::create(&source).unwrap();
    oversized.set_len(MAX_FILE_BYTES + 1).unwrap();
    assert!(matches!(
        import_image(&source, &target),
        Err(MenuIconError::FileTooLarge)
    ));
    assert_eq!(fs::read(&target).unwrap(), previous);

    let image = DynamicImage::ImageRgb8(RgbImage::new(MAX_IMAGE_DIMENSION + 1, 1));
    let mut bytes = Cursor::new(Vec::new());
    image.write_to(&mut bytes, ImageFormat::Png).unwrap();
    fs::write(&source, bytes.into_inner()).unwrap();
    assert!(matches!(
        import_image(&source, &target),
        Err(MenuIconError::ImageTooLarge)
    ));
    assert_eq!(fs::read(&target).unwrap(), previous);
}

#[test]
fn unsupported_format_is_rejected_without_creating_owned_data() {
    let directory = TempDir::new().unwrap();
    let source = directory.path().join("input.gif");
    fs::write(&source, b"GIF89a").unwrap();
    let target = directory.path().join("menu-icons/custom.png");
    assert!(matches!(
        import_image(&source, &target),
        Err(MenuIconError::UnsupportedFormat)
    ));
    assert!(!target.exists());
}

#[test]
fn reset_removes_only_the_owned_image_and_is_persistent() {
    let directory = TempDir::new().unwrap();
    let source = fixture(
        directory.path(),
        "input.png",
        ImageFormat::Png,
        [20, 40, 60],
    );
    let target = directory.path().join("menu-icons/custom.png");
    import_image(&source, &target).unwrap();
    reset_image(&target).unwrap();
    reset_image(&target).unwrap();
    assert!(source.is_file());
    let restarted = MenuIconManager::from_path(target);
    assert!(restarted.image_source().is_none());
    assert!(restarted.last_error().is_none());
}

#[test]
fn corrupt_owned_image_falls_back_with_a_readable_error() {
    let directory = TempDir::new().unwrap();
    let path = directory.path().join("custom.png");
    fs::write(&path, b"corrupt PNG").unwrap();
    let manager = MenuIconManager::from_path(path.clone());
    assert!(manager.image_source().is_none());
    assert!(matches!(
        manager.last_error(),
        Some(MenuIconError::StoredImageInvalid)
    ));
    assert!(!manager.last_error().unwrap().message().is_empty());
    assert_eq!(fs::read(path).unwrap(), b"corrupt PNG");
}

#[test]
fn actual_persist_failure_preserves_the_displayed_selection() {
    App::test((), |mut app| async move {
        let directory = TempDir::new().unwrap();
        let source = fixture(
            directory.path(),
            "input.png",
            ImageFormat::Png,
            [20, 40, 60],
        );
        let target = directory.path().join("custom.png");
        let previous = import_image(&source, &target).unwrap();
        let bytes = fs::read(&target).unwrap();
        let model = app.add_model(|_| MenuIconManager::from_path(target.clone()));
        let blocked_parent = directory.path().join("blocked");
        fs::write(&blocked_parent, b"not a directory").unwrap();
        let result = import_image(&source, &blocked_parent.join("custom.png")).map(Some);
        assert!(matches!(&result, Err(MenuIconError::Persist(_))));
        model.update(&mut app, |manager, ctx| {
            manager.finish_operation(result, ctx)
        });
        model.read(&app, |manager, _| {
            assert_eq!(manager.image_source(), Some(previous));
            assert!(matches!(
                manager.last_error(),
                Some(MenuIconError::Persist(_))
            ));
            assert!(!manager.is_importing());
        });
        assert_eq!(fs::read(target).unwrap(), bytes);
    });
}

#[test]
fn relative_owned_paths_are_never_written() {
    assert!(matches!(
        reset_image(Path::new("custom.png")),
        Err(MenuIconError::Persist(_))
    ));
}

#[test]
fn background_import_and_reset_notify_observers() {
    App::test((), |mut app| async move {
        let directory = TempDir::new().unwrap();
        let source = fixture(
            directory.path(),
            "input.png",
            ImageFormat::Png,
            [20, 40, 60],
        );
        let target = directory.path().join("menu-icons/custom.png");
        let model = app.add_model(|_| MenuIconManager::from_path(target.clone()));
        let (sender, receiver) = futures::channel::oneshot::channel();
        app.update(|ctx| {
            let mut sender = Some(sender);
            ctx.observe_model(&model, move |model, ctx| {
                if !model.read(ctx, |manager, _| manager.is_importing())
                    && let Some(sender) = sender.take()
                {
                    let _ = sender.send(());
                }
            });
        });
        model.update(&mut app, |manager, ctx| manager.import_image(source, ctx));
        receiver.await.unwrap();
        model.read(&app, |manager, _| {
            assert!(manager.image_source().is_some());
            assert!(manager.last_error().is_none());
        });
        assert!(target.is_file());

        let (sender, receiver) = futures::channel::oneshot::channel();
        app.update(|ctx| {
            let mut sender = Some(sender);
            ctx.observe_model(&model, move |model, ctx| {
                if !model.read(ctx, |manager, _| manager.is_importing())
                    && let Some(sender) = sender.take()
                {
                    let _ = sender.send(());
                }
            });
        });
        model.update(&mut app, MenuIconManager::reset);
        receiver.await.unwrap();
        model.read(&app, |manager, _| {
            assert!(manager.image_source().is_none());
            assert!(manager.last_error().is_none());
        });
        assert!(!target.exists());
    });
}

#[cfg(unix)]
#[test]
fn owned_directory_symlinks_are_never_written_or_reset() {
    let directory = TempDir::new().unwrap();
    let external = directory.path().join("external");
    fs::create_dir(&external).unwrap();
    let external_image = external.join("custom.png");
    fs::write(&external_image, b"preserve external file").unwrap();
    let owned_directory = directory.path().join("menu-icons");
    std::os::unix::fs::symlink(&external, &owned_directory).unwrap();
    let target = owned_directory.join("custom.png");
    let source = fixture(
        directory.path(),
        "input.png",
        ImageFormat::Png,
        [20, 40, 60],
    );
    assert!(matches!(
        import_image(&source, &target),
        Err(MenuIconError::Persist(_))
    ));
    assert!(matches!(
        reset_image(&target),
        Err(MenuIconError::Persist(_))
    ));
    assert!(MenuIconManager::from_path(target).image_source().is_none());
    assert_eq!(fs::read(external_image).unwrap(), b"preserve external file");
}
