//! Clarity.UI for Rust (DESIGN 7.9): Clarity's own UI framework, which draws
//! its windows itself with Skia, parts, layout, look and the person's input,
//! with no webview and no native controls. It reads the same window markup
//! (`.cxaml`) and the same resolved themes as every other version, passes
//! the same conformance cases, and runs a generated program through
//! `clarity_runtime::program::Program`.

pub mod app_files;
pub mod element;
pub mod fonts;
pub mod geometry;
pub mod grid;
pub mod input;
pub mod locale;
pub mod markup;
pub mod overlay;
pub mod painter;
pub mod parts;
pub mod screen;
pub mod shortcuts;
pub mod testing;
pub mod theme;
pub mod window;
pub mod xml;

#[cfg(feature = "desktop")]
pub mod desktop;

pub use app_files::AppFiles;
pub use element::{Cx, Element, ElementId, Node};
pub use geometry::Rect;
pub use input::{Key, KeyInput, Modifiers, PointerButton, PointerInput};
pub use overlay::{Clipboard, Overlay};
pub use painter::{Painter, TextAlign};
pub use parts::{Part, PartBase, PartHost};
pub use screen::Screen;
pub use theme::ThemeLook;
