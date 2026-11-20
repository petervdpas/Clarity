//! The C table gives what the crate gives, and keeps what went wrong.

fn made(api: &clarity_pdf::ClarityPdfApi, text: &str, sheet: &str, theme: &str) -> (i32, Vec<u8>) {
    let (mut out, mut len) = (std::ptr::null_mut(), 0usize);
    let code = (api.pdf)(text.as_ptr().cast(), text.len(), sheet.as_ptr().cast(), sheet.len(), theme.as_ptr().cast(), theme.len(), &mut out, &mut len);
    if code != 0 {
        return (code, Vec::new());
    }
    let file = unsafe { std::slice::from_raw_parts(out, len) }.to_vec();
    (api.free)(out, len);
    (code, file)
}

fn problem(api: &clarity_pdf::ClarityPdfApi) -> (i32, String) {
    let mut why = [0 as std::ffi::c_char; 256];
    let code = (api.problem)(why.as_mut_ptr(), why.len());
    (code, unsafe { std::ffi::CStr::from_ptr(why.as_ptr()) }.to_string_lossy().into_owned())
}

#[test]
fn the_table_makes_a_pdf_and_says_why_it_cannot() {
    let api = unsafe { &*clarity_pdf::clarity_pdf_api() };
    assert_eq!(2, api.interface);
    let (code, file) = made(api, "# Invoice\n\nThank you.", "", "");
    assert_eq!(0, code);
    assert_eq!(file, pdf::made("# Invoice\n\nThank you.", "", "").unwrap());
    assert_eq!(1, made(api, "text", "h1 { colr: red }", "").0);
    let (code, why) = problem(api);
    assert_eq!(1, code);
    assert!(why.contains("line 1") && why.contains("colr"), "{why}");
    assert_eq!(2, made(api, "text", "", "{ no").0);
    // A text that is no UTF-8 is refused, not read.
    let (mut out, mut len) = (std::ptr::null_mut(), 0usize);
    assert_eq!(4, (api.pdf)([0xffu8].as_ptr().cast(), 1, std::ptr::null(), 0, std::ptr::null(), 0, &mut out, &mut len));
}

#[test]
fn the_table_lays_out_a_report() {
    let api = unsafe { &*clarity_pdf::clarity_pdf_api() };
    let report = r#"<Report name="Note" paper="A5"><Data name="note" type="Note" /><ReportHeader height="20mm"><Field at="0,0" size="80,8" bind="note.title" /></ReportHeader></Report>"#;
    let data = r#"{ "note": { "title": "Hello" } }"#;
    let call = |report: &str, data: &str| {
        let (mut out, mut len) = (std::ptr::null_mut(), 0usize);
        let code = (api.report)(report.as_ptr().cast(), report.len(), data.as_ptr().cast(), data.len(), std::ptr::null(), 0, std::ptr::null(), 0, std::ptr::null(), 0, &mut out, &mut len);
        let file = if code == 0 { let f = unsafe { std::slice::from_raw_parts(out, len) }.to_vec(); (api.free)(out, len); f } else { Vec::new() };
        (code, file)
    };
    let (code, file) = call(report, data);
    assert_eq!(0, code);
    assert_eq!(file, pdf::report(report, data, "", "", "").unwrap());
    assert_eq!(5, call(report, "{}").0, "the data the report takes is missing");
    let (code, why) = problem(api);
    assert_eq!(5, code);
    assert!(why.contains("note"), "{why}");
}
