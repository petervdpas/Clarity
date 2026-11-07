//! The C table gives what the crate gives, and keeps what went wrong.

#[test]
fn the_table_resizes_and_says_why_it_cannot() {
    let api = unsafe { &*clarity_image::clarity_image_api() };
    assert_eq!(1, api.interface);
    let picture = pictures::encode(&image::RgbaImage::new(3, 2));
    let (mut out, mut len) = (std::ptr::null_mut(), 0usize);
    assert_eq!(0, (api.resize)(picture.as_ptr(), picture.len(), 6, 4, &mut out, &mut len));
    let made = unsafe { std::slice::from_raw_parts(out, len) }.to_vec();
    (api.free)(out, len);
    assert_eq!(Ok((6, 4)), pictures::size(&made));
    let (mut w, mut h) = (0u32, 0u32);
    assert_eq!(0, (api.size)(made.as_ptr(), made.len(), &mut w, &mut h));
    assert_eq!((6, 4), (w, h));
    assert_eq!(1, (api.resize)(b"none".as_ptr(), 4, 2, 2, &mut out, &mut len));
    let mut why = [0 as std::ffi::c_char; 128];
    assert_eq!(1, (api.problem)(why.as_mut_ptr(), why.len()));
    assert_eq!(2, (api.resize)(picture.as_ptr(), picture.len(), 0, 2, &mut out, &mut len));
}
