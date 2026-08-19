//! Handle `elda a <local artifact file>`.
//!
//! An operator points Elda at a release archive they already downloaded. Elda
//! identifies it by content magic, surveys it read-only, asks where it came
//! from, and writes a digest-pinned binary-lane recipe. Nothing is executed and
//! nothing is extracted before the operator has seen the survey.

use std::io::{self, IsTerminal, Write};
use std::path::Path;

use elda_types::{ArtifactAcquisition, ArtifactSurvey};

use crate::app::AppContext;
use elda_recipe::load_recipe;

use crate::app_install::ResolutionReport;
use crate::error::CoreError;

impl AppContext {
    /// Resolve a local artifact file, or `Ok(None)` when the file is not one.
    ///
    /// Returning `None` rather than erroring lets a non-artifact file fall
    /// through to the existing source-tree import path unchanged.
    pub(crate) fn resolve_local_artifact_target(
        &self,
        path: &Path,
        target: &str,
        request: &crate::app::ParsedInstallRequest,
    ) -> Result<Option<ResolutionReport>, CoreError> {
        if elda_build::local_artifact::identify(path).is_none() {
            return Ok(None);
        }

        let survey = elda_build::local_artifact::survey(path)
            .map_err(|error| CoreError::Operator(error.to_string()))?;
        let acquisition = self.resolve_acquisition(&survey, request)?;

        let recipes_dir = &self.database.layout().recipes_dir;
        let report = elda_recipe::write_local_artifact_recipe(
            recipes_dir,
            &survey,
            &acquisition,
            &self.metadata_import_options(request),
        )?;

        let recipe = load_recipe(recipes_dir, &report.recipe_name)?;
        let mut resolved =
            self.select_install_lane(target, recipe, request, Some(path.display().to_string()))?;
        resolved.generated_recipe_name = Some(report.recipe_name.clone());
        resolved.generated_recipe_dir = Some(report.recipe_dir.clone());

        Ok(Some(ResolutionReport::Single(Box::new(resolved))))
    }

    /// Decide where the artifact came from.
    ///
    /// `--from` wins. Otherwise an interactive operator is asked once, because
    /// the answer is what makes the package upgradeable; a non-interactive run
    /// records it as local-only rather than blocking.
    fn resolve_acquisition(
        &self,
        survey: &ArtifactSurvey,
        request: &crate::app::ParsedInstallRequest,
    ) -> Result<ArtifactAcquisition, CoreError> {
        if let Some(url) = &request.acquisition_url {
            return Ok(ArtifactAcquisition::Url(url.clone()));
        }
        if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
            return Ok(ArtifactAcquisition::LocalOnly);
        }
        prompt_for_acquisition(survey)
    }
}

/// Ask once for the download URL, reprompting inline on a bad answer.
fn prompt_for_acquisition(survey: &ArtifactSurvey) -> Result<ArtifactAcquisition, CoreError> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    let stdin = io::stdin();

    writeln!(
        stdout,
        "\nWhere was `{}` downloaded from? Paste the URL so Elda can re-fetch and upgrade it.",
        survey.file_name
    )?;
    writeln!(stdout, "Leave blank to keep it local-only (no upgrades).")?;

    loop {
        write!(stdout, ":: ")?;
        stdout.flush()?;

        let mut answer = String::new();
        if stdin.read_line(&mut answer)? == 0 {
            writeln!(stdout)?;
            return Ok(ArtifactAcquisition::LocalOnly);
        }
        let answer = answer.trim();
        if answer.is_empty() {
            return Ok(ArtifactAcquisition::LocalOnly);
        }
        if answer.starts_with("https://") || answer.starts_with("http://") {
            return Ok(ArtifactAcquisition::Url(answer.to_owned()));
        }
        writeln!(
            stdout,
            ":: expected an http(s) URL, or blank for local-only"
        )?;
    }
}
