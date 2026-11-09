//! 已停用功能的恢复视图；持有原描述，避免再次保存时丢失旧标签内容。

use warp_i18n::tr;
use warpui::elements::{Container, Flex, ParentElement, Text};
use warpui::{
    AppContext, Element, Entity, ModelHandle, SingletonEntity, TypedActionView, View, ViewContext,
    ViewHandle,
};

use super::{
    BackingView, DetachType, IPaneType, PaneConfiguration, PaneContent, PaneGroup, PaneId,
    PaneView, ShareableLink, ShareableLinkError, view,
};
use crate::app_state::{LeafContents, SettingsPaneSnapshot};
use crate::appearance::Appearance;
use crate::pane_group::PaneEvent;
use crate::pane_group::focus_state::PaneFocusHandle;

pub(in crate::pane_group) fn requires_unavailable_view(contents: &LeafContents) -> bool {
    match contents {
        LeafContents::Terminal(_) => false,
        LeafContents::Settings(SettingsPaneSnapshot::Local { current_page, .. }) => {
            !current_page.is_available_in_local_terminal()
        }
        _ => true,
    }
}

pub struct UnavailableFeaturePane {
    view: ViewHandle<PaneView<UnavailableFeatureView>>,
    pane_configuration: ModelHandle<PaneConfiguration>,
    original: LeafContents,
}

impl UnavailableFeaturePane {
    pub fn new<V: View>(original: LeafContents, ctx: &mut ViewContext<V>) -> Self {
        let child = ctx.add_typed_action_view(UnavailableFeatureView::new);
        let pane_configuration = child.as_ref(ctx).pane_configuration.clone();
        let view = ctx.add_typed_action_view(|ctx| {
            let pane_id = PaneId::new_from_ctx(IPaneType::UnavailableFeature, ctx);
            PaneView::new(pane_id, child, (), pane_configuration.clone(), ctx)
        });
        Self {
            view,
            pane_configuration,
            original,
        }
    }
}

impl PaneContent for UnavailableFeaturePane {
    fn id(&self) -> PaneId {
        PaneId::new(IPaneType::UnavailableFeature, &self.view)
    }

    fn attach(
        &self,
        _group: &PaneGroup,
        focus_handle: PaneFocusHandle,
        ctx: &mut ViewContext<PaneGroup>,
    ) {
        self.view
            .update(ctx, |view, ctx| view.set_focus_handle(focus_handle, ctx));
        let child = self.view.as_ref(ctx).child(ctx);
        let pane_id = self.id();
        ctx.subscribe_to_view(&child, move |group, _, event, ctx| {
            group.handle_pane_event(pane_id, event, ctx);
        });
        ctx.subscribe_to_view(&self.view, move |group, _, event, ctx| {
            group.handle_pane_view_event(pane_id, event, ctx);
        });
    }

    fn detach(&self, _group: &PaneGroup, _kind: DetachType, ctx: &mut ViewContext<PaneGroup>) {
        let child = self.view.as_ref(ctx).child(ctx);
        ctx.unsubscribe_to_view(&child);
        ctx.unsubscribe_to_view(&self.view);
    }

    fn snapshot(&self, _app: &AppContext) -> LeafContents {
        self.original.clone()
    }

    fn has_application_focus(&self, ctx: &mut ViewContext<PaneGroup>) -> bool {
        self.view.is_self_or_child_focused(ctx)
    }

    fn focus(&self, ctx: &mut ViewContext<PaneGroup>) {
        self.view
            .as_ref(ctx)
            .child(ctx)
            .update(ctx, BackingView::focus_contents);
    }

    fn shareable_link(
        &self,
        _ctx: &mut ViewContext<PaneGroup>,
    ) -> Result<ShareableLink, ShareableLinkError> {
        Ok(ShareableLink::Base)
    }

    fn pane_configuration(&self) -> ModelHandle<PaneConfiguration> {
        self.pane_configuration.clone()
    }

    fn is_pane_being_dragged(&self, ctx: &AppContext) -> bool {
        self.view.as_ref(ctx).is_being_dragged()
    }
}

pub struct UnavailableFeatureView {
    pane_configuration: ModelHandle<PaneConfiguration>,
}

impl UnavailableFeatureView {
    fn new(ctx: &mut ViewContext<Self>) -> Self {
        Self {
            pane_configuration: ctx.add_model(|_| {
                PaneConfiguration::new(tr(
                    "local_terminal.unavailable_pane.title",
                    "Feature unavailable",
                ))
            }),
        }
    }
}

impl Entity for UnavailableFeatureView {
    type Event = PaneEvent;
}

impl TypedActionView for UnavailableFeatureView {
    type Action = ();
    fn handle_action(&mut self, _action: &(), _ctx: &mut ViewContext<Self>) {}
}

impl View for UnavailableFeatureView {
    fn ui_name() -> &'static str {
        "UnavailableFeatureView"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let color = appearance
            .theme()
            .main_text_color(appearance.theme().surface_1())
            .into_solid();
        let content = Flex::column()
            .with_child(Text::new(
                tr("local_terminal.unavailable_pane.description", "This feature is unavailable in TinyWarp. The original tab and its data have been preserved. You can close this tab or continue using the terminal."),
                appearance.ui_font_family(), appearance.ui_font_size(),
            ).with_color(color).finish())
            .finish();
        Container::new(content)
            .with_padding_top(24.)
            .with_padding_bottom(24.)
            .with_padding_left(24.)
            .with_padding_right(24.)
            .finish()
    }
}

impl BackingView for UnavailableFeatureView {
    type PaneHeaderOverflowMenuAction = ();
    type CustomAction = ();
    type AssociatedData = ();

    fn handle_pane_header_overflow_menu_action(
        &mut self,
        _action: &(),
        _ctx: &mut ViewContext<Self>,
    ) {
    }
    fn close(&mut self, ctx: &mut ViewContext<Self>) {
        ctx.emit(PaneEvent::Close);
    }
    fn focus_contents(&mut self, ctx: &mut ViewContext<Self>) {
        ctx.focus_self();
    }

    fn render_header_content(
        &self,
        _ctx: &view::HeaderRenderContext<'_>,
        app: &AppContext,
    ) -> view::HeaderContent {
        view::HeaderContent::simple(self.pane_configuration.as_ref(app).title())
    }

    fn set_focus_handle(&mut self, _focus_handle: PaneFocusHandle, _ctx: &mut ViewContext<Self>) {}
}
