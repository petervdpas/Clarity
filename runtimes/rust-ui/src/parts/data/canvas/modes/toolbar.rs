//! A canvas mode's tools: the tool in hand, the brush and ink, the toolbox a
//! page holds or the toolbar drawn over the canvas, and importing a picture.

use super::*;

impl CanvasPart {
    /// The tools its toolbar shows: the canvas's own, of its mode, in the mode's order; all of them when it names none.
    pub fn tools_shown(&self) -> Vec<&'static str> {
        let all: &[&'static str] = match self.mode().as_str() {
            "paint" => &PAINT_TOOLS,
            "vector" => &VECTOR_TOOLS,
            // A diagram has tools only where it draws lines (`on connect`); else a press drags, as before.
            "objects" if self.handles("connect") => &OBJECT_TOOLS,
            _ => return Vec::new(),
        };
        let named = self.base.spec.names("tools");
        all.iter().copied().filter(|t| named.is_empty() || named.iter().any(|n| n == t)).collect()
    }

    /// The tool in hand: the one its state names, when the canvas has it, else its first.
    pub fn tool_in_hand(&self) -> &'static str {
        let tools = self.tools_shown();
        let named = self.base.state().and_then(|s| s.tool.clone()).unwrap_or_default();
        tools.iter().copied().find(|t| *t == named).or_else(|| tools.first().copied()).unwrap_or("")
    }

    /// How wide it paints or draws, in its units.
    pub fn brush(&self) -> f64 {
        self.base.state().and_then(|s| s.brush).map(|b| b as f64).or_else(|| self.base.spec.number("brush")).unwrap_or(2.0)
    }

    /// The color it paints or draws with: the theme's by name, or written out.
    pub fn ink(&self) -> String {
        self.base.state().and_then(|s| s.ink.clone()).filter(|i| !i.is_empty()).or_else(|| self.base.spec.text("ink").map(str::to_string)).unwrap_or_else(|| "text".to_string())
    }

    /// How its toolbox shows (`toolbox popup` or `toolbox slideout`): none without one, or on a canvas without a mode.
    pub fn toolbox_shows(&self) -> Option<super::super::toolbox::Shows> {
        if !matches!(self.mode().as_str(), "paint" | "vector" | "objects") {
            return None;
        }
        match self.base.spec.text("toolbox") {
            Some("popup") => Some(super::super::toolbox::Shows::Popup),
            Some("slideout") => Some(super::super::toolbox::Shows::Slideout),
            _ => None,
        }
    }

    /// Its toolbox, which shows while the person may paint on it.
    pub fn toolbox(&self) -> &std::rc::Rc<super::super::toolbox::Toolbox> {
        &self.modes.toolbox
    }

    /// Laid out: its toolbox opens, follows it, or goes when it may no longer be painted on.
    pub(in crate::parts::data::canvas) fn follow_toolbox(&mut self, cx: &mut Cx) {
        let shows = self.toolbox_shows().filter(|_| self.acts());
        let Some(name) = self.base.name().map(str::to_string) else { return };
        let link: (std::rc::Rc<dyn crate::parts::PartHost>, String) = (self.base.host.clone(), name);
        let tools = self.tools_shown();
        let paints = self.mode() == "paint";
        // The toolbox page's parts, placed in the canvas; without one, the built-in toolbox of its mode.
        let (parts, columns) = if self.base.spec.children.is_empty() {
            (super::super::toolbox::built_in(&self.mode(), &tools), 4)
        } else {
            (self.base.spec.children.clone(), self.base.spec.number("toolboxColumns").map_or(4, |c| c as i64))
        };
        self.follow_adjust(cx);
        self.check_layers();
        let following = super::super::toolbox::Following { shows, paints, canvas: self.base.node.bounds, link, parts, columns, written: (self.brush(), self.ink()) };
        self.modes.toolbox.follow(following, cx);
        match self.modes.toolbox.take_ask() {
            Some(super::super::toolbox::Ask::Import) => {
                let picker = crate::files::system();
                let title = locale::translate("importPicture", "Import a picture");
                let filter = crate::files::FileFilter { name: title.clone(), patterns: ["*.png", "*.jpg", "*.jpeg", "*.webp"].iter().map(|p| p.to_string()).collect() };
                let options = crate::files::PickerOptions { file_type: Some(filter), ..crate::files::PickerOptions::titled(&title) };
                cx.later(self.base.node.id(), move || Imported(picker.open_file(&options).and_then(|f| f.read())));
            }
            Some(super::super::toolbox::Ask::Export) => {
                let bytes = clarity_image::encode(&self.painted_pixels());
                let picker = crate::files::system();
                let title = locale::translate("exportPicture", "Export the picture");
                let name = format!("{}.png", self.base.name().unwrap_or("picture"));
                cx.later(self.base.node.id(), move || {
                    let options = crate::files::PickerOptions { suggested_name: Some(name), ..crate::files::PickerOptions::titled(&title) };
                    if let Some(file) = picker.save_file(&options) {
                        picker.write(&file, &bytes);
                    }
                    Exported
                });
            }
            Some(super::super::toolbox::Ask::Transform(t)) => self.transform(t, cx),
            Some(super::super::toolbox::Ask::Zoom(closer)) => self.zoom_step(closer, cx),
            Some(super::super::toolbox::Ask::Adjust) => self.open_adjust(cx),
            Some(super::super::toolbox::Ask::Layer(ask)) => self.layer_ask(ask, cx),
            // A memo's; a canvas's toolbox has no Cut, Copy and Paste row.
            Some(super::super::toolbox::Ask::Text(_)) | None => {}
        }
    }

    /// A picture file the person chose, laid on the paper: scaled to fit it, in the middle, over what is painted; one edit.
    pub fn import(&mut self, bytes: &[u8], cx: &mut Cx) {
        let Ok(picture) = clarity_image::decode(bytes) else {
            eprintln!("{}: the file is no picture", self.base.name().unwrap_or("canvas"));
            return;
        };
        let mut paper = self.painted_pixels();
        let (pw, ph) = (f64::from(paper.width()), f64::from(paper.height()));
        let fit = (pw / f64::from(picture.width())).min(ph / f64::from(picture.height())).min(1.0);
        let (w, h) = (((f64::from(picture.width()) * fit).round() as u32).max(1), ((f64::from(picture.height()) * fit).round() as u32).max(1));
        let scaled = clarity_image::ops::resize(&picture, w, h);
        clarity_image::ops::blend(&mut paper, &scaled, ((pw - f64::from(w)) / 2.0) as i64, ((ph - f64::from(h)) / 2.0) as i64, 100);
        let png = clarity_image::encode(&paper);
        *self.modes.painted.borrow_mut() = Some((self.painted_for(), Image::from_encoded(skia_safe::Data::new_copy(&png))));
        cx.invalidate();
        self.act(json!({ "stroke": FieldContent::New(png).value() }));
    }

    /// Its toolbar's buttons on the screen, in the order of its tools; none when its tools are in a toolbox.
    pub fn tool_buttons(&self, cx_fonts: &crate::fonts::Fonts, look: &crate::theme::ThemeLook) -> Vec<(&'static str, Rect)> {
        if self.toolbox_shows().is_some() {
            return Vec::new();
        }
        let b = self.base.node.bounds;
        let mut x = b.x + 6.0;
        self.tools_shown()
            .into_iter()
            .map(|tool| {
                let width = cx_fonts.measure(&tool_word(tool), "text", look.size("small"), false) + 16.0;
                let place = Rect::new(x, b.y + 6.0, width, TOOL_HEIGHT);
                x += width + 2.0;
                (tool, place)
            })
            .collect()
    }

    /// Its toolbar's buttons as last drawn, on the screen; none before it was drawn.
    pub fn toolbar_drawn(&self) -> Vec<(&'static str, Rect)> {
        self.modes.toolbar.borrow().clone()
    }

    pub(super) fn draw_toolbar(&self, p: &Painter) {
        let look = p.look;
        let in_hand = self.tool_in_hand();
        let usable = self.acts();
        let buttons = self.tool_buttons(p.fonts, look);
        *self.modes.toolbar.borrow_mut() = buttons.clone();
        for (tool, place) in buttons {
            // The tool in hand shows in the accent only while the person may use it.
            let chosen = usable && tool == in_hand;
            p.fill(place, look.color(if chosen { "accent" } else { "panel" }), look.radius("button"));
            p.stroke(place, look.color("border"), look.border(), look.radius("button"));
            p.text(&tool_word(tool), place, look.color(if chosen { "on_accent" } else { "text" }), look.size("small"), "text", false, TextAlign::Center);
        }
    }
}
