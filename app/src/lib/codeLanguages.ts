// The language of a file of a part's or a library's folder, by its name, as
// the code editor colors it.

export type CodeLanguage = "cpp" | "rust" | "go" | "csharp" | "cmake" | "toml" | "plain";

export function languageOfFile(path: string): CodeLanguage {
  const name = path.split("/").pop() ?? path;
  if (name === "CMakeLists.txt" || name.endsWith(".cmake")) return "cmake";
  if (name === "go.mod" || name === "go.sum") return "go";
  const dot = name.lastIndexOf(".");
  const extension = dot < 0 ? "" : name.slice(dot + 1).toLowerCase();
  switch (extension) {
    case "c":
    case "h":
    case "cc":
    case "cpp":
    case "cxx":
    case "hpp":
    case "hh":
      return "cpp";
    case "rs":
      return "rust";
    case "go":
      return "go";
    case "cs":
      return "csharp";
    case "toml":
      return "toml";
    default:
      return "plain";
  }
}
