use std::collections::HashMap;
use std::sync::Arc;

use fuzzy_match::FuzzyMatchResult;
use warpui::keymap::{BindingId, DescriptionContext};
use warpui::{AppContext, Entity, ModelContext, ModelHandle};

use crate::search::action::search_item::MatchedBinding;
use crate::search::binding_source::BindingSource;
use crate::search::command_palette::mixer::CommandPaletteItemAction;
use crate::search::data_source::{DataSourceSearchError, Query, QueryResult};
use crate::search::mixer::{DataSourceRunErrorWrapper, SyncDataSource};
use crate::util::bindings::{CommandBinding, match_binding_description};

/// Data source for [`CommandBinding`]s. Produces a list of in-app actions a user can currently
/// perform.
pub struct CommandBindingDataSource {
    searcher: Box<dyn ActionSearcher>,
}

impl CommandBindingDataSource {
    #[cfg(not(target_family = "wasm"))]
    pub fn new(binding_source: ModelHandle<BindingSource>, ctx: &mut ModelContext<Self>) -> Self {
        if warp_core::features::FeatureFlag::UseTantivySearch.is_enabled() {
            Self::new_full_text(binding_source, ctx)
        } else {
            Self::new_fuzzy(binding_source, ctx)
        }
    }

    #[cfg(target_family = "wasm")]
    pub fn new(binding_source: ModelHandle<BindingSource>, ctx: &mut ModelContext<Self>) -> Self {
        Self::new_fuzzy(binding_source, ctx)
    }

    #[cfg(not(target_family = "wasm"))]
    fn new_full_text(
        binding_source: ModelHandle<BindingSource>,
        ctx: &mut ModelContext<Self>,
    ) -> Self {
        ctx.observe(&binding_source, Self::on_binding_source_changed);

        let searcher = Box::new(full_text_searcher::FullTextActionSearcher::new());
        Self { searcher }
    }

    fn new_fuzzy(binding_source: ModelHandle<BindingSource>, ctx: &mut ModelContext<Self>) -> Self {
        ctx.observe(&binding_source, Self::on_binding_source_changed);

        let searcher = Box::new(FuzzyActionSearcher {
            all_bindings: Default::default(),
        });
        Self { searcher }
    }

    /// Returns a [`QueryResult`] for a binding with `binding_id`. `None` if no result was found
    /// with the given ID.
    pub fn query_result(
        &self,
        binding_id: BindingId,
    ) -> Option<QueryResult<CommandPaletteItemAction>> {
        self.searcher.bindings().get(&binding_id).map(|binding| {
            MatchedBinding::new(FuzzyMatchResult::no_match(), binding.clone()).into()
        })
    }

    fn on_binding_source_changed(
        &mut self,
        source: ModelHandle<BindingSource>,
        ctx: &mut ModelContext<Self>,
    ) {
        let (window_id, view_id, binding_filter_fn) = match source.as_ref(ctx) {
            BindingSource::None => return,
            BindingSource::View {
                window_id,
                view_id,
                binding_filter_fn,
            } => (*window_id, *view_id, binding_filter_fn.clone()),
        };

        *self.searcher.bindings_mut() = ctx
            .key_bindings_for_view(window_id, view_id)
            .into_iter()
            .filter_map(|lens| CommandBinding::from_lens(lens, ctx))
            .filter(|binding| binding_filter_fn.as_ref().is_none_or(|f| f(binding)))
            .map(Arc::new)
            .map(|binding| (binding.id, binding))
            .collect();

        self.searcher.build_index();
        ctx.emit(Event::IndexUpdated);
    }
}

impl SyncDataSource for CommandBindingDataSource {
    type Action = CommandPaletteItemAction;

    fn run_query(
        &self,
        query: &Query,
        _app: &AppContext,
    ) -> Result<Vec<QueryResult<Self::Action>>, DataSourceRunErrorWrapper> {
        self.searcher
            .search(&query.text.trim().to_lowercase())
            .map_err(|err| {
                let search_error = DataSourceSearchError::new(err.to_string());
                Box::new(search_error) as DataSourceRunErrorWrapper
            })
    }
}

pub enum Event {
    IndexUpdated,
}

impl Entity for CommandBindingDataSource {
    type Event = Event;
}

type SearcherAction = <CommandBindingDataSource as SyncDataSource>::Action;
type ActionMatch = (FuzzyMatchResult, Arc<CommandBinding>);

trait ActionSearcher {
    fn search_matches(&self, search_term: &str) -> anyhow::Result<Vec<ActionMatch>>;

    fn search(&self, search_term: &str) -> anyhow::Result<Vec<QueryResult<SearcherAction>>> {
        Ok(self
            .search_matches(search_term)?
            .into_iter()
            .map(|(match_result, binding)| MatchedBinding::new(match_result, binding).into())
            .collect())
    }

    fn build_index(&mut self);

    /// Set of cached bindings, keyed on [`BindingId`]. This is cached via the [`BindingSource`]
    /// model to ensure that we surface bindings to the user that were executable _before_ the
    /// command palette was opened.
    fn bindings(&self) -> &HashMap<BindingId, Arc<CommandBinding>>;

    fn bindings_mut(&mut self) -> &mut HashMap<BindingId, Arc<CommandBinding>>;
}

struct FuzzyActionSearcher {
    all_bindings: HashMap<BindingId, Arc<CommandBinding>>,
}

impl ActionSearcher for FuzzyActionSearcher {
    fn search_matches(&self, search_term: &str) -> anyhow::Result<Vec<ActionMatch>> {
        Ok(fuzzy_search_bindings(&self.all_bindings, search_term))
    }

    fn build_index(&mut self) {}

    fn bindings(&self) -> &HashMap<BindingId, Arc<CommandBinding>> {
        &self.all_bindings
    }

    fn bindings_mut(&mut self) -> &mut HashMap<BindingId, Arc<CommandBinding>> {
        &mut self.all_bindings
    }
}

fn fuzzy_search_bindings(
    bindings: &HashMap<BindingId, Arc<CommandBinding>>,
    search_term: &str,
) -> Vec<ActionMatch> {
    let search_term = search_term.to_lowercase();
    bindings
        .values()
        .filter(|binding| !is_excluded_binding(binding))
        .filter_map(|binding| {
            match_binding_description(
                &binding.description,
                DescriptionContext::Default,
                &search_term,
            )
            .map(|result| (result, binding.clone()))
        })
        .collect()
}

#[cfg(not(target_family = "wasm"))]
mod full_text_searcher {
    use std::collections::HashMap;
    use std::sync::Arc;

    use fuzzy_match::FuzzyMatchResult;
    use warp_search_core::define_search_schema;
    use warpui::keymap::{BindingId, DescriptionContext};

    use crate::search::action::data_source::{
        ActionMatch, ActionSearcher, fuzzy_search_bindings, is_excluded_binding,
    };
    use crate::search::searcher::{
        DEFAULT_MEMORY_BUDGET, SCORE_CONVERSION_FACTOR, SimpleFullTextSearcher,
    };
    use crate::util::bindings::CommandBinding;

    define_search_schema!(
        schema_name: ACTION_SEARCH_SCHEMA,
        config_name: ActionSearchConfig,
        search_doc: ActionDocument,
        identifying_doc: ActionIdDocument,
        search_fields: [action: 1.0, aliases: 1.0],
        id_fields: [id: u64]
    );

    pub(crate) struct FullTextActionSearcher {
        searcher: SimpleFullTextSearcher<ActionSearchConfig>,
        all_bindings: HashMap<BindingId, Arc<CommandBinding>>,
    }

    impl ActionSearcher for FullTextActionSearcher {
        fn search_matches(&self, search_term: &str) -> anyhow::Result<Vec<ActionMatch>> {
            if !search_term.is_ascii() {
                return Ok(fuzzy_search_bindings(&self.all_bindings, search_term));
            }
            if search_term.is_empty() {
                return Ok(self
                    .all_bindings
                    .values()
                    .filter(|binding| !is_excluded_binding(binding))
                    .map(|binding| (FuzzyMatchResult::no_match(), binding.clone()))
                    .collect());
            }

            let matched_bindings = self.searcher.search_id(&search_term.to_lowercase())?;
            Ok(matched_bindings
                .into_iter()
                .filter_map(|match_result| {
                    let binding = self
                        .all_bindings
                        .get(&BindingId(match_result.values.id as usize))?;
                    if is_excluded_binding(binding) {
                        return None;
                    }
                    let label = binding.description.in_context(DescriptionContext::Default);
                    let byte_indices = match_result.highlights.action;
                    let mut offset = 0;
                    let matched_indices = label
                        .chars()
                        .enumerate()
                        .filter_map(|(index, ch)| {
                            let end = offset + ch.to_lowercase().map(char::len_utf8).sum::<usize>();
                            let matched = byte_indices
                                .iter()
                                .any(|&byte| (offset..end).contains(&byte));
                            offset = end;
                            matched.then_some(index)
                        })
                        .collect();
                    Some((
                        FuzzyMatchResult {
                            score: (match_result.score * SCORE_CONVERSION_FACTOR) as i64,
                            matched_indices,
                        },
                        binding.clone(),
                    ))
                })
                .collect())
        }

        fn build_index(&mut self) {
            if self.rebuild_search_index().is_err() {
                warp_errors::report_error!("Failed to create search index writer for actions");
                self.clear_search_index();
            }
        }

        fn bindings(&self) -> &HashMap<BindingId, Arc<CommandBinding>> {
            &self.all_bindings
        }
        fn bindings_mut(&mut self) -> &mut HashMap<BindingId, Arc<CommandBinding>> {
            &mut self.all_bindings
        }
    }

    impl FullTextActionSearcher {
        pub(crate) fn new() -> Self {
            Self {
                searcher: ACTION_SEARCH_SCHEMA.create_searcher(DEFAULT_MEMORY_BUDGET),
                all_bindings: Default::default(),
            }
        }

        fn rebuild_search_index(&mut self) -> Result<(), anyhow::Error> {
            self.clear_search_index();
            let documents = self.all_bindings.iter().map(|(id, binding)| {
                let binding_description = binding
                    .description
                    .in_context(DescriptionContext::Default)
                    .to_lowercase();

                ActionDocument {
                    action: binding_description,
                    aliases: binding
                        .description
                        .search_aliases()
                        .join("\n")
                        .to_lowercase(),
                    id: id.0 as u64,
                }
            });
            self.searcher.build_index(documents)
        }

        fn clear_search_index(&mut self) {
            if self.searcher.clear_search_index().is_err() {
                // As a workaround, we can create a new index and replace the old one.
                self.searcher = ACTION_SEARCH_SCHEMA.create_searcher(DEFAULT_MEMORY_BUDGET);
            }
        }
    }
}

// Context on why the search_drive action is excluded can be seen here: https://github.com/warpdotdev/warp-internal/pull/11705
fn is_excluded_binding(binding: &CommandBinding) -> bool {
    binding.name == *"workspace:search_drive"
}

#[cfg(test)]
#[path = "data_source_tests.rs"]
mod tests;
