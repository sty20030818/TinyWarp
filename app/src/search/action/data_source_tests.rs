use warpui::keymap::BindingDescription;

use super::*;

fn searchers() -> Vec<Box<dyn ActionSearcher>> {
    let searchers: Vec<Box<dyn ActionSearcher>> = vec![Box::new(FuzzyActionSearcher {
        all_bindings: Default::default(),
    })];
    #[cfg(not(target_family = "wasm"))]
    let searchers = {
        let mut searchers = searchers;
        searchers.push(Box::new(full_text_searcher::FullTextActionSearcher::new()));
        searchers
    };
    searchers
}

fn binding(description: BindingDescription) -> Arc<CommandBinding> {
    let mut binding = CommandBinding::new("workspace:h1_settings".into(), String::new(), None);
    binding.description = description;
    Arc::new(binding)
}

#[test]
fn bilingual_action_queries_return_one_original_binding_without_alias_highlights() {
    for mut searcher in searchers() {
        let binding = binding(
            BindingDescription::new_preserve_case("打开设置")
                .with_search_alias("Open Settings")
                .with_search_alias("Settings"),
        );
        searcher.bindings_mut().insert(binding.id, binding.clone());
        searcher.build_index();

        for query in ["设置", "settings", "SETTINGS"] {
            let matches = searcher.search_matches(query).expect("动作查询应成功");
            assert_eq!(matches.len(), 1, "查询 {query} 只能返回一个动作");
            assert_eq!(matches[0].1.id, binding.id);
            if query.is_ascii() {
                assert!(
                    matches[0].0.matched_indices.is_empty(),
                    "英文别名下标不能用于高亮中文"
                );
            } else {
                assert_eq!(matches[0].0.matched_indices, vec![2, 3]);
            }
            let results = searcher.search(query).expect("命令面板应产生结果");
            let CommandPaletteItemAction::AcceptBinding { binding: matched } =
                results[0].accept_result()
            else {
                panic!("动作搜索必须保留原绑定动作");
            };
            assert_eq!(matched.id, binding.id);
        }
        assert_eq!(searcher.search_matches("").unwrap().len(), 1);
    }
}

#[test]
fn mixed_language_action_highlights_use_visible_character_indices() {
    for mut searcher in searchers() {
        let binding = binding(
            BindingDescription::new_preserve_case("打开 MCP 设置").with_search_alias("Settings"),
        );
        searcher.bindings_mut().insert(binding.id, binding.clone());
        searcher.build_index();
        for (query, expected_indices) in [
            ("mcp", vec![3, 4, 5]),
            ("设置", vec![7, 8]),
            ("打开M", vec![0, 1, 3]),
        ] {
            let matches = searcher.search_matches(query).unwrap();
            assert_eq!(matches.len(), 1, "查询 {query}");
            assert_eq!(matches[0].1.id, binding.id);
            assert_eq!(
                matches[0].0.matched_indices, expected_indices,
                "查询 {query}"
            );
        }
    }
}

#[test]
fn action_aliases_remain_searchable_after_dynamic_labels_are_materialized() {
    warpui::App::test((), |app| async move {
        let description = BindingDescription::new_preserve_case("向左移动标签页")
            .with_search_alias("Move Tab Left")
            .with_search_alias("Move Tab Up")
            .with_dynamic_override(|_| Some("向上移动标签页".into()));
        let description = app.read(|ctx| description.materialized(ctx));
        for mut searcher in searchers() {
            let binding = binding(description.clone());
            searcher.bindings_mut().insert(binding.id, binding.clone());
            searcher.build_index();
            for query in ["向上", "left", "up"] {
                let matches = searcher.search_matches(query).unwrap();
                assert_eq!(matches.len(), 1, "查询 {query}");
                assert_eq!(matches[0].1.id, binding.id);
            }
        }
    });
}
