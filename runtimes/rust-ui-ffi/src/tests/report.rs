//! A report's PDF from code (`pdf of Invoice with order = current`): Clarity.UI lays out a
//! report of the running program with its data, its texts and its resources.

use crate::files::{clarity_bytes_free, clarity_report_pdf};

fn running() {
    let report: &'static [u8] = br#"<Report name="Note" paper="A5"><Data name="note" type="Note" /><ReportHeader height="20mm"><Field at="0,0" size="80,8" bind="note.title" /></ReportHeader></Report>"#;
    let resources: &'static [(&'static str, &'static [u8])] = Box::leak(vec![("reports/Note.report", report)].into_boxed_slice());
    ui::app_files::set_running(ui::AppFiles { resources, model: "{}", ..Default::default() });
}

fn pdf(report: &str, data: &str) -> Result<Vec<u8>, String> {
    let (mut out, mut len) = (std::ptr::null_mut(), 0usize);
    let status = unsafe { clarity_report_pdf(report.as_ptr().cast(), report.len(), data.as_ptr().cast(), data.len(), &mut out, &mut len) };
    if status != crate::CLARITY_OK {
        return Err(super::last_error());
    }
    let made = unsafe { std::slice::from_raw_parts(out, len) }.to_vec();
    unsafe { clarity_bytes_free(out, len) };
    Ok(made)
}

#[test]
fn a_report_of_the_running_program_becomes_a_pdf() {
    running();
    let made = pdf("Note", r#"{ "note": { "title": "Hello" } }"#).unwrap();
    assert!(made.starts_with(b"%PDF-"));
    let why = pdf("Nothing", "{}").unwrap_err();
    assert!(why.contains("Nothing"), "{why}");
    let why = pdf("Note", "{").unwrap_err();
    assert!(why.contains("JSON"), "{why}");
}
