//! How Emscripten links the browser program: a module the worker (or the page) loads
//! (`ClarityUi`), WebGL2, the worker's functions the host imports
//! (js/library.js) and the entry points the worker calls.

const EXPORTED: &[&str] = &[
    "_main",
    "_malloc",
    "_free",
    "_clarity_ui_start",
    "_clarity_ui_reload",
    "_clarity_ui_layout",
    "_clarity_ui_resize",
    "_clarity_ui_dark",
    "_clarity_ui_pointer",
    "_clarity_ui_wheel",
    "_clarity_ui_key",
    "_clarity_ui_text",
    "_clarity_ui_compose",
    "_clarity_ui_pasted",
    "_clarity_ui_sound_ended",
    "_clarity_ui_song_said",
    "_clarity_ui_file_chosen",
    "_clarity_ui_answered",
    "_clarity_ui_frame",
    "_clarity_ui_next_wake",
    "_clarity_ui_step",
    "_clarity_ui_shot",
    "_clarity_ui_access",
    "_clarity_ui_act",
    "_clarity_ui_string_free",
];

fn main() {
    println!("cargo:rerun-if-changed=js/library.js");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("emscripten") {
        return;
    }
    let library = std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("js").join("library.js");
    let args = [
        "-sMODULARIZE=1".to_string(),
        "-sEXPORT_NAME=ClarityUi".to_string(),
        // A worker, or the page itself where a worker cannot draw with WebGL2 (WebKitGTK).
        "-sENVIRONMENT=web,worker".to_string(),
        "-sALLOW_MEMORY_GROWTH=1".to_string(),
        "-sSTACK_SIZE=2MB".to_string(),
        "-sMIN_WEBGL_VERSION=2".to_string(),
        "-sMAX_WEBGL_VERSION=2".to_string(),
        "-sEXPORTED_RUNTIME_METHODS=GL,UTF8ToString,stringToNewUTF8,HEAPU8".to_string(),
        format!("-sEXPORTED_FUNCTIONS={}", EXPORTED.join(",")),
        format!("--js-library={}", library.display()),
        // Skia is C++: since Emscripten 6.0.6 emcc links C++'s libraries only when asked.
        "-sDEFAULT_TO_CXX".to_string(),
    ];
    for arg in args {
        println!("cargo:rustc-link-arg-bins={arg}");
    }
}
