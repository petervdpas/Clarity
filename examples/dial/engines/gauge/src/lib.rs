//! The Gauge engine: the needle of a dial whose scale runs over 240
//! degrees, from 210 (none) to -30 (the maximum), and the colour it shows.

clarity_engine::interface!();

// angle(speed, maximum) -> integer: the needle's angle in degrees, kept on the scale.
clarity_engine::function!(angle, |(speed, maximum): (i64, i64)| {
    if maximum <= 0 {
        return Err("a dial needs a maximum above 0".to_string());
    }
    let speed = speed.clamp(0, maximum);
    Ok(210 - speed * 240 / maximum)
});

// zone(speed) -> text: the theme colour of the needle at that speed.
clarity_engine::function!(zone, |(speed,): (i64,)| Ok(match speed {
    ..120 => "accent",
    120..160 => "warning",
    _ => "error",
}));
