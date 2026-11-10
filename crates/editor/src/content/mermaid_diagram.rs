use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;

#[cfg(feature = "mermaid")]
use bytes::Bytes;
use warpui_core::assets::asset_cache::{
    AssetCache, AssetSource, AssetState, AsyncAssetId, AsyncAssetType,
};
use warpui_core::image_cache::ImageType;
use warpui_core::units::{IntoPixels, Pixels};
use warpui_core::{AppContext, SingletonEntity};

use crate::render::layout::TextLayout;
use crate::render::model::{BlockSpacing, ImageBlockConfig};

const DEFAULT_MERMAID_HEIGHT_LINE_MULTIPLIER: f32 = 10.0;
const FAILED_MERMAID_HEIGHT_LINE_MULTIPLIER: f32 = 2.0;

struct MermaidDiagramAsset;

impl AsyncAssetType for MermaidDiagramAsset {}

// 语言标记解析与渲染能力分离，保持已有 Markdown 分类及源码往返。
pub fn is_mermaid_diagram(lang: &str) -> bool {
    let lang_lower = lang.to_lowercase();
    lang_lower == "mermaid" || lang_lower.starts_with("mermaid ")
}

pub fn mermaid_asset_source(source: &str) -> AssetSource {
    let source = source.to_string();
    let mut hasher = DefaultHasher::new();
    source.hash(&mut hasher);
    let id = format!("configured:{:x}", hasher.finish());
    #[cfg(feature = "mermaid")]
    let fetch_source = source;

    AssetSource::Async {
        id: AsyncAssetId::new::<MermaidDiagramAsset>(id),
        fetch: Arc::new(move || {
            #[cfg(feature = "mermaid")]
            let source = fetch_source.clone();
            Box::pin(async move {
                #[cfg(feature = "mermaid")]
                {
                    mermaid_to_svg::render_mermaid_to_svg(&source, None)
                        .map(|svg| Bytes::from(svg.into_bytes()))
                        .map_err(Into::into)
                }
                #[cfg(not(feature = "mermaid"))]
                {
                    // 加载失败沿用源码代码块回退，不生成空图冒充成功。
                    anyhow::bail!("此构建未包含 Mermaid 图表渲染器")
                }
            })
        }),
    }
}

pub fn mermaid_diagram_layout(
    source: &str,
    layout: &TextLayout,
    spacing: BlockSpacing,
    app: &AppContext,
) -> (AssetSource, ImageBlockConfig) {
    let asset_source = mermaid_asset_source(source);
    let config = mermaid_diagram_config(&asset_source, layout, spacing, app);

    (asset_source, config)
}

fn mermaid_diagram_config(
    asset_source: &AssetSource,
    layout: &TextLayout,
    spacing: BlockSpacing,
    app: &AppContext,
) -> ImageBlockConfig {
    let max_width = layout.max_width() - spacing.x_axis_offset();
    let (width, height) = mermaid_diagram_size(asset_source, max_width, app).unwrap_or_else(|| {
        let height = layout.rich_text_styles().base_line_height()
            * mermaid_diagram_fallback_height_line_multiplier(asset_source, app).into_pixels();
        (max_width, height)
    });
    ImageBlockConfig {
        width,
        height,
        spacing,
    }
}

fn mermaid_diagram_fallback_height_line_multiplier(
    asset_source: &AssetSource,
    app: &AppContext,
) -> f32 {
    let asset_cache = AssetCache::as_ref(app);
    match asset_cache.load_asset::<ImageType>(asset_source.clone()) {
        AssetState::FailedToLoad(_) => FAILED_MERMAID_HEIGHT_LINE_MULTIPLIER,
        AssetState::Loading { .. } | AssetState::Loaded { .. } | AssetState::Evicted => {
            DEFAULT_MERMAID_HEIGHT_LINE_MULTIPLIER
        }
    }
}
fn mermaid_diagram_size(
    asset_source: &AssetSource,
    max_width: Pixels,
    app: &AppContext,
) -> Option<(Pixels, Pixels)> {
    let asset_cache = AssetCache::as_ref(app);
    let AssetState::Loaded { data } = asset_cache.load_asset::<ImageType>(asset_source.clone())
    else {
        return None;
    };
    let ImageType::Svg { svg } = data.as_ref() else {
        return None;
    };
    let intrinsic_size = svg.size();
    let intrinsic_width = intrinsic_size.width();
    let intrinsic_height = intrinsic_size.height();
    if intrinsic_width <= 0. || intrinsic_height <= 0. {
        return None;
    }
    let width = max_width;
    let height = Pixels::new(width.as_f32() * intrinsic_height / intrinsic_width);
    Some((width, height))
}

#[cfg(test)]
#[path = "mermaid_diagram_tests.rs"]
mod tests;
