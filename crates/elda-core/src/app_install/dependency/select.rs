use crate::app::{AppContext, DependencyCandidate};
use crate::error::CoreError;

impl AppContext {
    pub(crate) fn select_unique_dependency_candidate(
        &self,
        candidates: &[DependencyCandidate],
        context: &str,
    ) -> Result<DependencyCandidate, CoreError> {
        Self::select_preferred_dependency_candidate(candidates.to_vec(), context)?.ok_or_else(
            || {
                CoreError::Recipe(elda_recipe::RecipeError::InvalidInput(format!(
                    "no available {context} exists in the current install scope"
                )))
            },
        )
    }

    pub(crate) fn select_preferred_dependency_candidate(
        candidates: Vec<DependencyCandidate>,
        context: &str,
    ) -> Result<Option<DependencyCandidate>, CoreError> {
        if candidates.is_empty() {
            return Ok(None);
        }

        let installed_candidates = candidates
            .iter()
            .filter(|candidate| candidate.installed)
            .cloned()
            .collect::<Vec<_>>();
        if installed_candidates.len() == 1 {
            return Ok(installed_candidates.into_iter().next());
        }
        if installed_candidates.len() > 1 {
            return Err(ambiguous_dependency_error(context, &installed_candidates));
        }

        if candidates.len() == 1 {
            return Ok(candidates.into_iter().next());
        }

        if let Some(candidate) = select_by_priority_and_version(&candidates, context)? {
            return Ok(Some(candidate));
        }

        Err(ambiguous_dependency_error(context, &candidates))
    }
}

fn select_by_priority_and_version(
    candidates: &[DependencyCandidate],
    context: &str,
) -> Result<Option<DependencyCandidate>, CoreError> {
    if candidates
        .iter()
        .any(|candidate| candidate.source_priority.is_none())
    {
        return Ok(None);
    }

    let best_priority = candidates
        .iter()
        .filter_map(|candidate| candidate.source_priority)
        .min()
        .expect("prioritized candidate selection should not run on an empty slice");
    let prioritized = candidates
        .iter()
        .filter(|candidate| candidate.source_priority == Some(best_priority))
        .cloned()
        .collect::<Vec<_>>();
    if prioritized.len() == 1 {
        return Ok(prioritized.into_iter().next());
    }
    if prioritized
        .iter()
        .any(|candidate| candidate.candidate_version.is_none())
    {
        return Err(ambiguous_dependency_error(context, &prioritized));
    }

    let best_version = prioritized
        .iter()
        .filter_map(|candidate| candidate.candidate_version.clone())
        .max()
        .expect("version-aware provider selection should have candidate versions");
    let version_winners = prioritized
        .into_iter()
        .filter(|candidate| candidate.candidate_version.as_ref() == Some(&best_version))
        .collect::<Vec<_>>();
    if version_winners.len() == 1 {
        return Ok(version_winners.into_iter().next());
    }

    Err(ambiguous_dependency_error(context, &version_winners))
}

fn ambiguous_dependency_error(context: &str, candidates: &[DependencyCandidate]) -> CoreError {
    CoreError::Recipe(elda_recipe::RecipeError::InvalidInput(format!(
        "ambiguous {context}; candidates are `{}`",
        candidates
            .iter()
            .map(|candidate| candidate.target.as_str())
            .collect::<Vec<_>>()
            .join("`, `")
    )))
}
