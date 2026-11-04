//! The accessibility tree as AccessKit's nodes, which the desktop hands to
//! the system's screen reader (AT-SPI, UI Automation, the macOS
//! accessibility API): a whole tree the first time, then only the nodes
//! that changed since the tree before (`update`).

pub use accesskit;

use accesskit::{Action, ActionData, ActionRequest, Affine, Invalid, Node, NodeId, Role as KitRole, Toggled, TreeId, TreeInfo, TreeUpdate};

use super::{AccessAction, AccessNode, AccessTree, Role, ROOT};

/// AccessKit's role for a node's.
pub fn role(role: Role) -> KitRole {
    match role {
        Role::Window => KitRole::Window,
        Role::Dialog => KitRole::Dialog,
        Role::AlertDialog => KitRole::AlertDialog,
        Role::Group => KitRole::Group,
        Role::Form => KitRole::Form,
        Role::Heading => KitRole::Heading,
        Role::Label => KitRole::Label,
        Role::Document => KitRole::Document,
        Role::Button => KitRole::Button,
        Role::Link => KitRole::Link,
        Role::CheckBox => KitRole::CheckBox,
        Role::Switch => KitRole::Switch,
        Role::RadioGroup => KitRole::RadioGroup,
        Role::Radio => KitRole::RadioButton,
        Role::TextBox => KitRole::TextInput,
        Role::TextArea => KitRole::MultilineTextInput,
        Role::Password => KitRole::PasswordInput,
        Role::SpinButton => KitRole::SpinButton,
        Role::Slider => KitRole::Slider,
        Role::ComboBox => KitRole::ComboBox,
        Role::ListBox => KitRole::ListBox,
        Role::Option => KitRole::ListBoxOption,
        Role::Date => KitRole::DateInput,
        Role::Color => KitRole::ColorWell,
        Role::Image => KitRole::Image,
        Role::Canvas => KitRole::Canvas,
        Role::ProgressBar => KitRole::ProgressIndicator,
        Role::Table => KitRole::Table,
        Role::TreeGrid => KitRole::TreeGrid,
        Role::Row => KitRole::Row,
        Role::Cell => KitRole::Cell,
        Role::ColumnHeader => KitRole::ColumnHeader,
        Role::TabList => KitRole::TabList,
        Role::Tab => KitRole::Tab,
        Role::MenuBar => KitRole::MenuBar,
        Role::Menu => KitRole::Menu,
        Role::MenuItem => KitRole::MenuItem,
        Role::MenuItemCheckBox => KitRole::MenuItemCheckBox,
        Role::Toolbar => KitRole::Toolbar,
        Role::Complementary => KitRole::Complementary,
        Role::ContentInfo => KitRole::ContentInfo,
        Role::Status => KitRole::Status,
        Role::Log => KitRole::Log,
        Role::Separator => KitRole::Splitter,
    }
}

fn action(action: AccessAction) -> Action {
    match action {
        AccessAction::Focus => Action::Focus,
        AccessAction::Click => Action::Click,
        AccessAction::SetValue => Action::SetValue,
        AccessAction::Increment => Action::Increment,
        AccessAction::Decrement => Action::Decrement,
        AccessAction::ScrollIntoView => Action::ScrollIntoView,
        AccessAction::Expand => Action::Expand,
        AccessAction::Collapse => Action::Collapse,
    }
}

/// AccessKit's node for one of the tree's.
pub fn node(n: &AccessNode) -> Node {
    let mut out = Node::new(role(n.role));
    if !n.name.is_empty() {
        out.set_label(n.name.clone());
    }
    match &n.value {
        Some(value) => out.set_value(value.clone()),
        // A label's and a status's text is their value to the system's screen reader (AT-SPI reads it there).
        None if matches!(n.role, Role::Label | Role::Status) && !n.name.is_empty() => out.set_value(n.name.clone()),
        None => {}
    }
    let s = &n.states;
    let description = match (&s.invalid, &n.description) {
        (Some(why), _) => Some(why.clone()),
        (None, d) => d.clone(),
    };
    if let Some(description) = description {
        out.set_description(description);
    }
    let b = n.bounds;
    out.set_bounds(accesskit::Rect { x0: b.x, y0: b.y, x1: b.x + b.width, y1: b.y + b.height });
    for a in &n.actions {
        out.add_action(action(*a));
    }
    out.set_children(n.children.iter().map(|c| NodeId(*c)).collect::<Vec<_>>());
    if s.disabled {
        out.set_disabled();
    }
    if s.readonly {
        out.set_read_only();
    }
    if s.required {
        out.set_required();
    }
    if s.modal {
        out.set_modal();
    }
    if s.invalid.is_some() {
        out.set_invalid(Invalid::True);
    }
    if let Some(checked) = s.checked {
        out.set_toggled(if checked { Toggled::True } else { Toggled::False });
    }
    if let Some(selected) = s.selected {
        out.set_selected(selected);
    }
    if let Some(expanded) = s.expanded {
        out.set_expanded(expanded);
    }
    if let Some(numeric) = &n.numeric {
        if let Some(v) = numeric.value {
            out.set_numeric_value(v);
        }
        if let Some(v) = numeric.min {
            out.set_min_numeric_value(v);
        }
        if let Some(v) = numeric.max {
            out.set_max_numeric_value(v);
        }
        if let Some(v) = numeric.step {
            out.set_numeric_value_step(v);
        }
    }
    if let Some(level) = n.level {
        out.set_level(level);
    }
    out
}

/// The roles whose value is text the person types: the system's screen
/// reader reads it from a text run under the node, with the caret at its end.
fn types(n: &AccessNode) -> bool {
    matches!(n.role, Role::TextBox | Role::TextArea | Role::SpinButton | Role::Date | Role::Color) || (n.role == Role::ComboBox && n.actions.contains(&AccessAction::SetValue))
}

/// The text run of a node whose value is typed text, and the node with it under it.
fn with_run(n: &AccessNode, mut made: Node) -> (Node, Option<(NodeId, Node)>) {
    let (Some(text), Some(owner)) = (n.value.as_ref().filter(|_| types(n)), super::owner_of(n.id)) else { return (made, None) };
    let run_id = NodeId(super::virtual_id(owner, 0xF_FFFF));
    let mut run = Node::new(KitRole::TextRun);
    run.set_value(text.clone());
    run.set_character_lengths(text.chars().map(|c| c.len_utf8() as u8).collect::<Vec<_>>());
    let b = n.bounds;
    run.set_bounds(accesskit::Rect { x0: b.x, y0: b.y, x1: b.x + b.width, y1: b.y + b.height });
    made.push_child(run_id);
    let end = accesskit::TextPosition { node: run_id, character_index: text.chars().count() };
    made.set_text_selection(accesskit::TextSelection { anchor: end, focus: end });
    (made, Some((run_id, run)))
}

/// What the screen reader is told: every node and the tree's root when there
/// was no tree before, else only the nodes new or changed since `before`
/// (a node whose children went changes too, which drops them). `scale` is
/// the system's pixels per unit of the screen.
pub fn update(before: Option<&AccessTree>, now: &AccessTree, scale: f64) -> TreeUpdate {
    let mut nodes = Vec::new();
    let old: std::collections::HashMap<u64, &AccessNode> = before.map(|b| b.nodes.iter().map(|n| (n.id, n)).collect()).unwrap_or_default();
    for n in &now.nodes {
        let same = old.get(&n.id).is_some_and(|old| *old == n);
        if same && !(n.id == ROOT && before.is_none()) {
            continue;
        }
        let mut made = node(n);
        if n.id == ROOT {
            made.set_transform(Affine::scale(scale));
        }
        let (made, run) = with_run(n, made);
        nodes.push((NodeId(n.id), made));
        nodes.extend(run);
    }
    let tree = before.is_none().then(|| {
        let mut info = TreeInfo::new(NodeId(ROOT));
        info.toolkit_name = Some("Clarity.UI".into());
        info.toolkit_version = Some(env!("CARGO_PKG_VERSION").into());
        info
    });
    TreeUpdate { nodes, tree, tree_id: TreeId::ROOT, focus: NodeId(now.focus) }
}

/// What a screen reader asked for, in the tree's words: the node, the
/// action, and the value for SetValue (a text, or a number as text).
pub fn request(request: &ActionRequest) -> Option<(u64, AccessAction, Option<String>)> {
    let asked = match request.action {
        Action::Focus => AccessAction::Focus,
        Action::Click => AccessAction::Click,
        Action::SetValue => AccessAction::SetValue,
        Action::Increment => AccessAction::Increment,
        Action::Decrement => AccessAction::Decrement,
        Action::ScrollIntoView => AccessAction::ScrollIntoView,
        Action::Expand => AccessAction::Expand,
        Action::Collapse => AccessAction::Collapse,
        _ => return None,
    };
    let value = match &request.data {
        Some(ActionData::Value(text)) => Some(text.to_string()),
        Some(ActionData::NumericValue(n)) => Some(n.to_string()),
        _ => None,
    };
    Some((request.target_node.0, asked, value))
}
