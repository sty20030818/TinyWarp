use warpui::App;

use super::*;
use crate::settings_view::settings_page::FilteredPageType;
use crate::test_util::settings::initialize_settings_for_tests;

#[test]
fn shell_and_ssh_search_keeps_page_title_and_scopes_matching_widgets() {
    App::test((), |mut app| async move {
        initialize_settings_for_tests(&mut app);
        app.read(|ctx| {
            let mut page = WarpifyPageView::build_page(ctx);
            for (query, expected_widgets) in [("识别命令", 1), ("ControlMaster", 1), ("", 3)] {
                assert!(page.update_filter(query, ctx).is_truthy());
                let FilteredPageType::Categorized {
                    categories, title, ..
                } = page.get_filtered()
                else {
                    panic!("Shell 与 SSH 增强应使用分类页面");
                };
                assert!(title.is_some(), "过滤具体设置时保留页面标题");
                assert_eq!(
                    categories
                        .iter()
                        .map(|category| category.widgets.len())
                        .sum::<usize>(),
                    expected_widgets,
                );
            }
        });
    });
}
