use std::fs::{self, File};
use std::io::{self, Cursor, Read, Write};
use std::path::{Path, PathBuf};

use image::{ImageDecoder as _, ImageFormat, ImageReader, Limits};
use tempfile::NamedTempFile;
use thiserror::Error;
use warpui::assets::asset_cache::{AssetSource, LocalFileContentVersion};
use warpui::{Entity, ModelContext, SingletonEntity};

const MAX_FILE_BYTES: u64 = 10 * 1024 * 1024;
const MAX_IMAGE_DIMENSION: u32 = 8192;
const MAX_IMAGE_PIXELS: u64 = 16_000_000;
const ICON_SIZE: u32 = 128;

#[derive(Debug, Error)]
pub enum MenuIconError {
    #[error("unsupported menu image format")]
    UnsupportedFormat,
    #[error("menu image exceeds file size limit")]
    FileTooLarge,
    #[error("menu image exceeds pixel limit")]
    ImageTooLarge,
    #[error("invalid menu image: {0}")]
    InvalidImage(#[source] image::ImageError),
    #[error("cannot read menu image: {0}")]
    Read(#[source] io::Error),
    #[error("cannot save menu image: {0}")]
    Persist(#[source] io::Error),
    #[error("saved menu image is unavailable or invalid")]
    StoredImageInvalid,
}

impl MenuIconError {
    pub fn message_key(&self) -> &'static str {
        match self {
            Self::UnsupportedFormat => "menu_icon.error.unsupported_format",
            Self::FileTooLarge => "menu_icon.error.file_too_large",
            Self::ImageTooLarge => "menu_icon.error.image_too_large",
            Self::InvalidImage(_) => "menu_icon.error.invalid_image",
            Self::Read(_) => "menu_icon.error.read",
            Self::Persist(_) => "menu_icon.error.persist",
            Self::StoredImageInvalid => "menu_icon.error.saved_image",
        }
    }

    pub fn message(&self) -> &'static str {
        match self {
            Self::UnsupportedFormat => "请选择 PNG 或 JPEG 图片。",
            Self::FileTooLarge => "图片不能超过 10 MB。",
            Self::ImageTooLarge => {
                "图片尺寸过大，请选择不超过 1600 万像素、单边不超过 8192 像素的图片。"
            }
            Self::InvalidImage(_) => "无法解码这张图片，请选择有效的 PNG 或 JPEG 图片。",
            Self::Read(_) => "无法读取图片，请检查文件是否存在及访问权限。",
            Self::Persist(_) => "无法保存菜单图片，原有选择已保留。请检查本地数据目录的访问权限。",
            Self::StoredImageInvalid => {
                "已保存的菜单图片无法读取，已显示默认图标。请重新选择图片或恢复默认。"
            }
        }
    }
}

pub struct MenuIconManager {
    path: PathBuf,
    source: Option<AssetSource>,
    importing: bool,
    last_error: Option<MenuIconError>,
}

impl MenuIconManager {
    pub fn new(ctx: &mut ModelContext<Self>) -> Self {
        let path = warp_core::paths::data_dir().join("menu-icons/custom.png");
        if crate::local_terminal::is_enabled(ctx) {
            Self::from_path(path)
        } else {
            Self {
                path,
                source: None,
                importing: false,
                last_error: None,
            }
        }
    }

    fn from_path(path: PathBuf) -> Self {
        let (source, last_error) = match load_stored_image(&path) {
            Ok(source) => (source, None),
            Err(error) => (None, Some(error)),
        };
        Self {
            path,
            source,
            importing: false,
            last_error,
        }
    }

    pub fn image_source(&self) -> Option<AssetSource> {
        self.source.clone()
    }

    pub fn is_importing(&self) -> bool {
        self.importing
    }

    pub fn last_error(&self) -> Option<&MenuIconError> {
        self.last_error.as_ref()
    }

    pub fn import_image(&mut self, path: PathBuf, ctx: &mut ModelContext<Self>) {
        if self.importing {
            return;
        }
        self.importing = true;
        self.last_error = None;
        ctx.notify();
        let target = self.path.clone();
        let _ = ctx.spawn(
            async move { import_image(&path, &target).map(Some) },
            Self::finish_operation,
        );
    }

    pub fn reset(&mut self, ctx: &mut ModelContext<Self>) {
        if self.importing {
            return;
        }
        self.importing = true;
        self.last_error = None;
        ctx.notify();
        let path = self.path.clone();
        let _ = ctx.spawn(
            async move { reset_image(&path).map(|()| None) },
            Self::finish_operation,
        );
    }

    fn finish_operation(
        &mut self,
        result: Result<Option<AssetSource>, MenuIconError>,
        ctx: &mut ModelContext<Self>,
    ) {
        match result {
            Ok(source) => self.source = source,
            Err(error) => self.last_error = Some(error),
        }
        self.importing = false;
        ctx.notify();
    }
}

impl Entity for MenuIconManager {
    type Event = ();
}

impl SingletonEntity for MenuIconManager {}

fn read_image(path: &Path, max_bytes: u64) -> Result<Vec<u8>, MenuIconError> {
    if !fs::metadata(path).map_err(MenuIconError::Read)?.is_file() {
        return Err(MenuIconError::Read(io::Error::other("not a regular file")));
    }
    let file = File::open(path).map_err(MenuIconError::Read)?;
    let metadata = file.metadata().map_err(MenuIconError::Read)?;
    if !metadata.is_file() {
        return Err(MenuIconError::Read(io::Error::other("not a regular file")));
    }
    if metadata.len() > max_bytes {
        return Err(MenuIconError::FileTooLarge);
    }
    let mut bytes = Vec::new();
    file.take(max_bytes + 1)
        .read_to_end(&mut bytes)
        .map_err(MenuIconError::Read)?;
    if bytes.len() as u64 > max_bytes {
        return Err(MenuIconError::FileTooLarge);
    }
    Ok(bytes)
}

fn decode_image(bytes: &[u8], max_dimension: u32) -> Result<image::DynamicImage, MenuIconError> {
    let format = image::guess_format(bytes).map_err(MenuIconError::InvalidImage)?;
    if !matches!(format, ImageFormat::Png | ImageFormat::Jpeg) {
        return Err(MenuIconError::UnsupportedFormat);
    }
    let mut limits = Limits::default();
    limits.max_image_width = Some(max_dimension);
    limits.max_image_height = Some(max_dimension);
    limits.max_alloc = Some(128 * 1024 * 1024);
    let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
    reader.limits(limits);
    let mut decoder = reader.into_decoder().map_err(image_error)?;
    let (width, height) = decoder.dimensions();
    if u64::from(width) * u64::from(height) > MAX_IMAGE_PIXELS {
        return Err(MenuIconError::ImageTooLarge);
    }
    let orientation = decoder.orientation().map_err(image_error)?;
    let mut image = image::DynamicImage::from_decoder(decoder).map_err(image_error)?;
    image.apply_orientation(orientation);
    Ok(image)
}

fn image_error(error: image::ImageError) -> MenuIconError {
    match error {
        image::ImageError::Limits(_) => MenuIconError::ImageTooLarge,
        other => MenuIconError::InvalidImage(other),
    }
}

fn checked_path(path: &Path) -> Result<&str, MenuIconError> {
    if !path.is_absolute() {
        return Err(MenuIconError::Persist(io::Error::other(
            "data directory unavailable",
        )));
    }
    path.to_str()
        .ok_or_else(|| MenuIconError::Persist(io::Error::other("invalid data directory")))
}

fn load_stored_image(path: &Path) -> Result<Option<AssetSource>, MenuIconError> {
    checked_path(path)?;
    check_owned_parent(path)?;
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Ok(metadata) if metadata.file_type().is_file() => {}
        _ => return Err(MenuIconError::StoredImageInvalid),
    }
    let bytes = read_image(path, 1024 * 1024).map_err(|_| MenuIconError::StoredImageInvalid)?;
    if image::guess_format(&bytes).ok() != Some(ImageFormat::Png) {
        return Err(MenuIconError::StoredImageInvalid);
    }
    let image = decode_image(&bytes, ICON_SIZE).map_err(|_| MenuIconError::StoredImageInvalid)?;
    if image.width() != ICON_SIZE || image.height() != ICON_SIZE {
        return Err(MenuIconError::StoredImageInvalid);
    }
    Ok(Some(AssetSource::LocalFile {
        path: checked_path(path)?.to_owned(),
        content_version: Some(
            LocalFileContentVersion::for_path(path).ok_or(MenuIconError::StoredImageInvalid)?,
        ),
    }))
}

fn import_image(source: &Path, target: &Path) -> Result<AssetSource, MenuIconError> {
    let target_name = checked_path(target)?.to_owned();
    let image = decode_image(&read_image(source, MAX_FILE_BYTES)?, MAX_IMAGE_DIMENSION)?;
    let image = image.resize_to_fill(ICON_SIZE, ICON_SIZE, image::imageops::FilterType::Lanczos3);
    let mut bytes = Cursor::new(Vec::new());
    image
        .write_to(&mut bytes, ImageFormat::Png)
        .map_err(image_error)?;
    let parent = target
        .parent()
        .ok_or_else(|| MenuIconError::Persist(io::Error::other("missing data directory")))?;
    fs::create_dir_all(parent).map_err(MenuIconError::Persist)?;
    if !fs::symlink_metadata(parent)
        .map_err(MenuIconError::Persist)?
        .file_type()
        .is_dir()
    {
        return Err(MenuIconError::Persist(io::Error::other(
            "menu image directory is not owned",
        )));
    }
    let mut file = NamedTempFile::new_in(parent).map_err(MenuIconError::Persist)?;
    file.write_all(bytes.get_ref())
        .map_err(MenuIconError::Persist)?;
    file.as_file().sync_all().map_err(MenuIconError::Persist)?;
    // 原子替换保留临时文件的元数据，提交后不再执行可能失败的状态更新。
    let version = LocalFileContentVersion::for_path(file.path())
        .ok_or_else(|| MenuIconError::Persist(io::Error::other("cannot version menu image")))?;
    file.persist(target)
        .map_err(|error| MenuIconError::Persist(error.error))?;
    Ok(AssetSource::LocalFile {
        path: target_name,
        content_version: Some(version),
    })
}

fn reset_image(path: &Path) -> Result<(), MenuIconError> {
    checked_path(path)?;
    check_owned_parent(path)?;
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(MenuIconError::Persist(error)),
    }
}

fn check_owned_parent(path: &Path) -> Result<(), MenuIconError> {
    let parent = path
        .parent()
        .ok_or_else(|| MenuIconError::Persist(io::Error::other("missing data directory")))?;
    match fs::symlink_metadata(parent) {
        Ok(metadata) if metadata.file_type().is_dir() => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(MenuIconError::Persist(error)),
        Ok(_) => Err(MenuIconError::Persist(io::Error::other(
            "menu image directory is not owned",
        ))),
    }
}

#[cfg(test)]
#[path = "menu_icon_tests.rs"]
mod tests;
