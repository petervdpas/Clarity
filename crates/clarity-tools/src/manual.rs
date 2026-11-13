//! Clarity's manual: Markdown topics in manual, one file per topic,
//! named by its id (`code.queries.md`). A topic's first line is its title
//! (`# Queries`), the sentence under it its summary, and the rest the page.
//! People read the files as they are; the agent gets the list of titles and
//! summaries, and reads a page when it needs one. manual/README.md lists
//! the topics in the same order for people.

/// Every topic, in the order of the manual. A topic file that is not listed
/// here fails the tests, and so does a listed one that the index leaves out.
const TOPICS: [(&str, &str); 56] = [
    ("start.overview", include_str!("../../../manual/start.overview.md")),
    ("start.how-it-works", include_str!("../../../manual/start.how-it-works.md")),
    ("start.first-project", include_str!("../../../manual/start.first-project.md")),
    ("start.complete-example", include_str!("../../../manual/start.complete-example.md")),
    ("project.file", include_str!("../../../manual/project.file.md")),
    ("project.run-build", include_str!("../../../manual/project.run-build.md")),
    ("project.signin", include_str!("../../../manual/project.signin.md")),
    ("project.agents", include_str!("../../../manual/project.agents.md")),
    ("setup.tools", include_str!("../../../manual/setup.tools.md")),
    ("design.windows", include_str!("../../../manual/design.windows.md")),
    ("design.frame", include_str!("../../../manual/design.frame.md")),
    ("design.parts", include_str!("../../../manual/design.parts.md")),
    ("design.custom-parts", include_str!("../../../manual/design.custom-parts.md")),
    ("design.sound", include_str!("../../../manual/design.sound.md")),
    ("design.video", include_str!("../../../manual/design.video.md")),
    ("design.text", include_str!("../../../manual/design.text.md")),
    ("parts.text", include_str!("../../../manual/parts.text.md")),
    ("parts.input", include_str!("../../../manual/parts.input.md")),
    ("parts.containers", include_str!("../../../manual/parts.containers.md")),
    ("parts.data", include_str!("../../../manual/parts.data.md")),
    ("parts.graphics", include_str!("../../../manual/parts.graphics.md")),
    ("parts.media", include_str!("../../../manual/parts.media.md")),
    ("parts.time", include_str!("../../../manual/parts.time.md")),
    ("parts.toolbox", include_str!("../../../manual/parts.toolbox.md")),
    ("parts.signin", include_str!("../../../manual/parts.signin.md")),
    ("code.modules", include_str!("../../../manual/code.modules.md")),
    ("code.handlers", include_str!("../../../manual/code.handlers.md")),
    ("code.statements", include_str!("../../../manual/code.statements.md")),
    ("code.expressions", include_str!("../../../manual/code.expressions.md")),
    ("code.functions", include_str!("../../../manual/code.functions.md")),
    ("code.commands", include_str!("../../../manual/code.commands.md")),
    ("code.queries", include_str!("../../../manual/code.queries.md")),
    ("code.toasts", include_str!("../../../manual/code.toasts.md")),
    ("code.libraries", include_str!("../../../manual/code.libraries.md")),
    ("code.web-apis", include_str!("../../../manual/code.web-apis.md")),
    ("code.formats", include_str!("../../../manual/code.formats.md")),
    ("code.engines", include_str!("../../../manual/code.engines.md")),
    ("data.entities", include_str!("../../../manual/data.entities.md")),
    ("data.settings", include_str!("../../../manual/data.settings.md")),
    ("data.sources", include_str!("../../../manual/data.sources.md")),
    ("data.migrations", include_str!("../../../manual/data.migrations.md")),
    ("data.diagrams", include_str!("../../../manual/data.diagrams.md")),
    ("data.pictures-files", include_str!("../../../manual/data.pictures-files.md")),
    ("languages", include_str!("../../../manual/languages.md")),
    ("themes", include_str!("../../../manual/themes.md")),
    ("stacks.rust", include_str!("../../../manual/stacks.rust.md")),
    ("stacks.csharp", include_str!("../../../manual/stacks.csharp.md")),
    ("stacks.go", include_str!("../../../manual/stacks.go.md")),
    ("stacks.c", include_str!("../../../manual/stacks.c.md")),
    ("stacks.cpp", include_str!("../../../manual/stacks.cpp.md")),
    ("ai.agent", include_str!("../../../manual/ai.agent.md")),
    ("ai.connections", include_str!("../../../manual/ai.connections.md")),
    ("ai.mcp", include_str!("../../../manual/ai.mcp.md")),
    ("ai.your-agent", include_str!("../../../manual/ai.your-agent.md")),
    ("vault", include_str!("../../../manual/vault.md")),
    ("app.keys", include_str!("../../../manual/app.keys.md")),
];

/// A page of the manual.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Topic {
    pub id: &'static str,
    /// The first heading, without its `#`.
    pub title: &'static str,
    /// The sentence under the title.
    pub summary: &'static str,
    /// The whole page, as written.
    pub text: &'static str,
}

fn read(id: &'static str, text: &'static str) -> Topic {
    let mut lines = text.lines();
    let title = lines.next().unwrap_or_default().trim_start_matches('#').trim();
    let summary = lines.map(str::trim).find(|l| !l.is_empty()).unwrap_or_default();
    Topic { id, title, summary, text }
}

/// Every topic, in the order of the manual.
pub fn topics() -> Vec<Topic> {
    TOPICS.iter().map(|(id, text)| read(id, text)).collect()
}

/// The topic `id`.
pub fn topic(id: &str) -> Option<Topic> {
    TOPICS.iter().find(|(name, _)| *name == id.trim()).map(|(id, text)| read(id, text))
}

/// A page for the manual's window: a topic.
pub fn page(id: &str) -> Result<String, String> {
    topic(id).map(|t| t.text.to_string()).ok_or_else(|| format!("the manual has no page {}", id.trim()))
}

/// The topics, for the manual's window.
/// The manual as a list for the agent: one line per topic with its id,
/// title and summary.
pub fn index() -> String {
    // The summary up to its colon, where one has the details after it: enough to choose a page by.
    let short = |summary: &str| summary.split_once(": ").map_or_else(|| summary.to_string(), |(head, _)| format!("{head}."));
    topics().iter().map(|t| format!("- {}: {}. {}\n", t.id, t.title, short(&t.summary))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn folder() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../manual")
    }

    #[test]
    fn every_topic_file_is_in_the_manual_and_the_index() {
        let mut files: Vec<String> = std::fs::read_dir(folder())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
            .filter(|n| n.ends_with(".md") && n != "README.md")
            .map(|n| n.trim_end_matches(".md").to_string())
            .collect();
        files.sort();
        let mut listed: Vec<String> = TOPICS.iter().map(|(id, _)| id.to_string()).collect();
        listed.sort();
        assert_eq!(files, listed, "manual and the list of topics differ");

        // The index for people names every topic, in the same order.
        let readme = std::fs::read_to_string(folder().join("README.md")).unwrap();
        // Its list entries, `- [Title](id.md): summary`, not the links of its introduction.
        let linked: Vec<&str> = readme
            .lines()
            .filter_map(|l| l.strip_prefix("- ["))
            .filter_map(|l| l.split("](").nth(1))
            .filter_map(|l| l.split(".md)").next())
            .collect();
        let order: Vec<&str> = TOPICS.iter().map(|(id, _)| *id).collect();
        assert_eq!(linked, order, "manual/README.md must link every topic, in the manual's order");
    }

    #[test]
    fn every_topic_has_a_title_and_a_summary() {
        for t in topics() {
            assert!(t.text.starts_with("# "), "{} starts with its title", t.id);
            assert!(!t.title.is_empty() && !t.summary.is_empty(), "{} has a title and a summary", t.id);
            assert!(!t.summary.starts_with('#') && t.summary.ends_with('.'), "{}: the summary is one sentence under the title: {}", t.id, t.summary);
        }
    }

    #[test]
    fn every_link_leads_somewhere() {
        for t in topics() {
            for (at, _) in t.text.match_indices("](") {
                // A `](` inside inline code on its line is code, not a link.
                let line_start = t.text[..at].rfind('\n').map_or(0, |i| i + 1);
                if t.text[line_start..at].matches('`').count() % 2 == 1 {
                    continue;
                }
                let target = t.text[at + 2..].split(')').next().unwrap_or_default();
                if target.starts_with("http") || target.starts_with('#') {
                    continue;
                }
                let file = target.split('#').next().unwrap_or_default();
                assert!(folder().join(file).exists(), "{}: the link {target} leads nowhere", t.id);
            }
        }
    }


    #[test]
    fn the_manual_is_written_in_the_house_style() {
        for t in topics() {
            for bad in ['\u{2014}', '\u{2013}', '\u{2026}', '\u{201C}', '\u{201D}', '\u{2018}', '\u{2019}'] {
                assert!(!t.text.contains(bad), "{} holds {bad:?}", t.id);
            }
        }
    }


    #[test]
    fn a_page_is_a_topic() {
        assert!(page("code.queries").unwrap().starts_with("# "));
        assert!(page("reference").is_err(), "the language reference is the manual's pages now");
        assert!(page("teleport").unwrap_err().contains("teleport"));
    }

    #[test]
    fn a_topic_is_found_by_its_id() {
        assert_eq!(topic("code.queries").unwrap().id, "code.queries");
        assert!(topic("nothing").is_none());
        assert!(index().lines().count() == TOPICS.len() && index().contains("- start.overview: "));
    }
}
