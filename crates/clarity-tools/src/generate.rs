//! Generating a project's source tree for its stack, as `clarity gen` and
//! the agents' `generate` tool do: each part's folder follows its
//! declaration, then every program of the stack is written into the output
//! folder. It writes code; it builds and runs nothing.

use std::path::Path;

use clarity_spec::Severity;

/// Generates the project of `project_file` into `out`, or into the folder
/// its `output` setting names. Gives the lines that say what was written,
/// or the lines that say why nothing was.
pub fn generate(project_file: &Path, out: Option<&Path>) -> Result<Vec<String>, Vec<String>> {
    if !project_file.is_file() {
        return Err(vec![format!("{} is not a project file; give one like shop.clarity", project_file.display())]);
    }
    let dir = clarity_project::module_dir(project_file);
    let load = || clarity_project::load_modules(&dir).map_err(|err| vec![format!("cannot read {}: {err}", dir.display())]);
    let mut lines = Vec::new();
    // As Generate in Clarity: each part's folder follows its declaration first (DESIGN 5.19).
    let written = clarity_gen::parts::write_part_folders(&dir, &load()?).map_err(|err| vec![format!("cannot write the parts' folders: {err}")])?;
    lines.extend(written.iter().map(|path| format!("wrote {}", path.display())));
    let project = load()?;
    let file = clarity_project::load(project_file);
    let Some(settings) = file.settings.filter(|_| file.problems.is_empty()) else {
        return Err(file.problems.iter().map(|problem| format!("{}: {problem}", project_file.display())).collect());
    };
    let out = out.map_or_else(|| clarity_project::out_dir(project_file, &settings), Path::to_path_buf);
    let built = clarity_project::build_settings(&dir, &settings);
    let mut programs = clarity_gen::programs(&project, &built).map_err(|errors| {
        let mut why: Vec<String> = project.diagnostics.iter().filter(|d| d.severity == Severity::Error).map(|d| d.to_string()).collect();
        why.extend(errors);
        why
    })?;
    // A program served to a browser carries Clarity.UI's browser files (round B of DESIGN 7.9):
    // CLARITY_UI_BROWSER's, the fetched ones, else the ones Clarity's repository built.
    if clarity_gen::web_browser::needs_library(&programs) {
        let repository = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
        clarity_gen::web_browser::local_folder(Some(repository))
            .and_then(|folder| clarity_gen::web_browser::with_library(&mut programs, &folder))
            .map_err(|why| vec![why])?;
    }
    let changed = clarity_gen::write_programs(&out, &programs, settings.stack.backend, &settings.keeps).map_err(|err| vec![format!("cannot write {}: {err}", out.display())])?;
    for program in &programs {
        let dir = clarity_project::target_dir(&out, program.target);
        lines.push(format!("{} file(s) in {}", program.files.len(), dir.display()));
    }
    lines.push(format!("{changed} changed"));
    Ok(lines)
}
