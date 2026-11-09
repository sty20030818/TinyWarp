use super::*;

#[test]
fn appearance_search_keeps_english_keywords_and_adds_chinese() {
    let theme_terms = CreateCustomThemeWidget::default();
    let font_terms = TerminalFontWidget::default();
    let opacity_terms = WindowOpacityWidget::default();
    for (terms, english, chinese) in [
        (theme_terms.search_terms(), "custom theme", "主题"),
        (font_terms.search_terms(), "font", "字体"),
        (opacity_terms.search_terms(), "opacity", "透明度"),
    ] {
        assert!(super::super::settings_page::search_terms_match(
            terms, english
        ));
        assert!(super::super::settings_page::search_terms_match(
            terms, chinese
        ));
    }
}
