use super::{query_filter_display_name, query_filter_placeholder};
use crate::search::QueryFilter;

#[test]
fn localized_history_filter_keeps_its_original_query_syntax() {
    let filter = QueryFilter::History;
    assert_eq!(query_filter_display_name(filter), "历史记录");
    assert_eq!(query_filter_placeholder(filter), "搜索历史记录");
    assert_eq!(filter.filter_atom().primary_text, "history:");
    assert_eq!(filter.display_name(), "history");
}
