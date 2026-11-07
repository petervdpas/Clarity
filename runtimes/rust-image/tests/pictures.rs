//! The picture functions give the pixels every stack gives (conformance/picture-functions.json).

use image::RgbaImage;
use serde_json::{Value, json};

fn picture(v: &Value) -> RgbaImage {
    let (w, h) = (v["width"].as_u64().unwrap() as u32, v["height"].as_u64().unwrap() as u32);
    let hex = v["pixels"].as_str().unwrap();
    let bytes: Vec<u8> = (0..hex.len()).step_by(2).map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap()).collect();
    RgbaImage::from_raw(w, h, bytes).unwrap()
}

fn written(p: &RgbaImage) -> Value {
    json!({ "width": p.width(), "height": p.height(), "pixels": p.as_raw().iter().map(|b| format!("{b:02x}")).collect::<String>() })
}

fn call(case: &Value) -> RgbaImage {
    let p = clarity_image::encode(&picture(&case["picture"]));
    let a = case["args"].as_array().unwrap();
    let n = |i: usize| a[i].as_f64().unwrap();
    let out = match case["function"].as_str().unwrap() {
        "crop" => clarity_image::crop(&p, n(0) as i64, n(1) as i64, n(2) as i64, n(3) as i64),
        "flip" => clarity_image::flip(&p, a[0].as_bool().unwrap()),
        "rotate" => clarity_image::rotate(&p, n(0)),
        "resize" => clarity_image::resize(&p, n(0) as i64, n(1) as i64),
        "adjust" => clarity_image::adjust(&p, n(0), n(1), n(2), n(3)),
        "blend" => clarity_image::blend(&p, &clarity_image::encode(&picture(&case["over"])), n(0) as i64, n(1) as i64, n(2) as i64),
        other => panic!("no function {other}"),
    };
    clarity_image::decode(&out.unwrap()).unwrap()
}

#[test]
fn the_picture_functions_give_the_pixels_every_stack_gives() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../conformance/picture-functions.json");
    let mut doc: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let write = std::env::var("CLARITY_WRITE_PICTURES").is_ok();
    for case in doc["cases"].as_array_mut().unwrap() {
        let got = written(&call(case));
        if case.get("result").is_none() && write {
            case["result"] = got;
            continue;
        }
        assert_eq!(case["result"], got, "{}", case["name"]);
    }
    if write {
        std::fs::write(path, serde_json::to_string_pretty(&doc).unwrap() + "\n").unwrap();
    }
}

#[test]
fn a_picture_that_is_none_is_refused_and_sizes_are_checked() {
    assert!(matches!(clarity_image::resize(b"no picture", 2, 2), Err(clarity_image::Problem::NotAPicture(_))));
    let p = clarity_image::encode(&RgbaImage::new(3, 2));
    assert_eq!(Err(clarity_image::Problem::BadSize), clarity_image::resize(&p, 0, 2));
    assert_eq!(Ok((3, 2)), clarity_image::size(&p));
}

#[test]
fn a_fill_paints_the_area_of_near_colours_it_reaches() {
    use image::Rgba;
    let (w, k, r) = (Rgba([255, 255, 255, 255]), Rgba([0, 0, 0, 255]), Rgba([255, 0, 0, 255]));
    // A black wall down the middle keeps the right half white.
    let mut p = RgbaImage::from_fn(3, 2, |x, _| if x == 1 { k } else { w });
    clarity_image::ops::fill(&mut p, 0, 0, r, 0.0);
    assert_eq!((*p.get_pixel(0, 0), *p.get_pixel(0, 1), *p.get_pixel(1, 0), *p.get_pixel(2, 0)), (r, r, k, w));
}
