#[cfg(feature = "word_scoring")]
mod scoring;
#[cfg(feature = "word_scoring")]
mod word_list;
#[cfg(feature = "word_scoring")]
pub use scoring::{WordDb, is_word, natural_language_words_score};

pub fn check_if_token_has_shell_syntax(word: &str) -> bool {
    // List of special characters from https://mywiki.wooledge.org/BashGuide/SpecialCharacters.
    // Note that here we check if the word contains whitespace first to make sure we are running
    // on a single token.
    !word.contains(' ')
        && word.contains([
            '$', '=', '{', '}', '[', ']', '>', '<', '*', '~', '&', '(', ')', '|', '/', '-',
        ])
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
