use super::check_if_token_has_shell_syntax;

#[test]
fn shell_syntax_is_available_without_word_scoring() {
    for token in ["$HOME", "key=value", "~/project", "--help", "a|b", "[x]"] {
        assert!(check_if_token_has_shell_syntax(token), "{token}");
    }
    for token in ["hello", "中文", "hello world", "two words/with/path"] {
        assert!(!check_if_token_has_shell_syntax(token), "{token}");
    }
}

#[cfg(feature = "word_scoring")]
#[test]
fn full_build_keeps_dictionary_scoring() {
    use std::borrow::Cow;

    use super::{WordDb, is_word, natural_language_words_score};

    assert!(is_word("hello", WordDb::English));
    assert!(is_word("git", WordDb::Command));
    assert_eq!(natural_language_words_score(vec![], false), 0);
    assert_eq!(
        natural_language_words_score(vec![Cow::Borrowed("hello")], false),
        1
    );
    assert_eq!(
        natural_language_words_score(vec![Cow::Borrowed("git"), Cow::Borrowed("hello")], false),
        1
    );
}
