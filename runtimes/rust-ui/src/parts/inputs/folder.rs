//! A folder part (X2): a folder field's path, typed or chosen with the
//! system's folder dialog; the path part (`path.rs`) does it, choosing a
//! folder.

use std::rc::Rc;

use crate::locale;
use crate::markup::PartSpec;
use crate::parts::inputs::path::{Chooses, PathPart};
use crate::parts::{Part, PartHost};

/// What the button does, in Clarity's own words: its name and the title of the picker.
pub fn choose_name() -> String {
    locale::translate("chooseFolder", "Choose a folder")
}

/// What a folder part says where nothing picks a folder (a browser, whose program and folders are on the server).
pub fn type_name() -> String {
    locale::translate("typeServerFolder", "Type the path of a folder on the server")
}

pub fn make(spec: PartSpec, host: Rc<dyn PartHost>) -> Box<dyn Part> {
    Box::new(PathPart::new(spec, host, Chooses::Folder))
}
