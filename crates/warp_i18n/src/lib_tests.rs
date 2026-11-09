use super::{Catalog, EMBEDDED_CATALOG, load_catalog, tr};

#[test]
fn embedded_catalog_is_valid() {
    Catalog::parse(EMBEDDED_CATALOG).expect("内嵌文案目录必须有效");
}

#[test]
fn translated_text_is_returned_for_existing_key() {
    let catalog = Catalog::parse(r#"{"settings.appearance":"外观"}"#).unwrap();
    assert_eq!(catalog.lookup("settings.appearance", "Appearance"), "外观");
}

#[test]
fn missing_translation_keeps_english_literal() {
    let catalog = Catalog::parse("{}").unwrap();
    assert_eq!(catalog.lookup("missing", "Appearance"), "Appearance");
}

#[test]
fn public_lookup_falls_back_to_a_static_english_reference() {
    let text: &'static str = tr("__test_missing_translation__", "Font (default)");
    assert_eq!(text, "Font (default)");
}

#[test]
fn catalog_rejects_duplicate_keys_including_escaped_spellings() {
    for source in [
        r#"{"label":"甲","label":"乙"}"#,
        r#"{"label":"甲","l\u0061bel":"乙"}"#,
    ] {
        assert!(Catalog::parse(source).is_err(), "重复键必须失败: {source}");
    }
}

#[test]
fn catalog_rejects_blank_keys_or_translations() {
    for source in [r#"{"":"甲"}"#, r#"{"label":""}"#, r#"{"label":" \n\t"}"#] {
        assert!(
            Catalog::parse(source).is_err(),
            "空白文案必须失败: {source}"
        );
    }
}

#[test]
fn catalog_rejects_malformed_json_and_non_string_values() {
    for source in [
        "{",
        "[]",
        "null",
        r#"{"label":42}"#,
        r#"{"label":"甲"} false"#,
    ] {
        assert!(
            Catalog::parse(source).is_err(),
            "非法目录必须失败: {source}"
        );
    }
}

#[test]
fn unusable_catalog_falls_back_to_english_without_partial_translations() {
    for source in ["{", r#"{"label":"甲","invalid":""}"#] {
        assert_eq!(load_catalog(source).lookup("label", "Label"), "Label");
    }
}
