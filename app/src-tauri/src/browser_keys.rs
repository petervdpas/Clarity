//! WebView2's own keys: F5 and Ctrl+R reload the page, Ctrl+P prints, Ctrl+F
//! finds, Alt+Left goes back. In Clarity those keys are Clarity's (DESIGN 5.28),
//! or nothing, so the webview's browser keys are off. The page still hears
//! every key; only the webview's own action is gone.

/// Turns off the browser keys of a window's webview. WebKitGTK and WKWebView
/// have none of their own, so elsewhere than Windows there is nothing to do.
pub fn turn_off(window: &tauri::WebviewWindow) {
    #[cfg(windows)]
    {
        let done = window.with_webview(|webview| {
            use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Settings3;
            use windows_core::Interface;
            // SAFETY: COM calls on the webview's own controller, run on its thread by with_webview.
            let off = unsafe { webview.controller().CoreWebView2().and_then(|core| core.Settings()).and_then(|settings| settings.cast::<ICoreWebView2Settings3>()).and_then(|settings| settings.SetAreBrowserAcceleratorKeysEnabled(false)) };
            if let Err(err) = off {
                eprintln!("clarity: the webview's own keys stay on: {err}");
            }
        });
        if let Err(err) = done {
            eprintln!("clarity: the webview's own keys stay on: {err}");
        }
    }
    #[cfg(not(windows))]
    let _ = window;
}
