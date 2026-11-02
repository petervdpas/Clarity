//! Custom parts on C# (DESIGN 5.19, 7.9): a part declared with a .NET
//! library (a NuGet package or a DLL). They come with Clarity.UI's custom
//! controls; until then a project with one says so when it is generated.

use clarity_spec::Project;
use clarity_spec::code::{PartDecl, PartLibrary};

/// The project's parts drawn by .NET libraries.
pub fn dotnet_parts(project: &Project) -> Vec<&PartDecl> {
    project.files.iter().flat_map(|f| &f.parts).filter(|p| matches!(p.library, Some(PartLibrary::Nuget { .. } | PartLibrary::Dll { .. }))).collect()
}
