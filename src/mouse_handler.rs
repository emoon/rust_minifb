use MouseMode;

fn clamp(v: f32, lb: f32, ub: f32) -> f32 {
    f32::min(f32::max(v, lb), ub)
}

pub fn get_pos(
    mode: MouseMode,
    mx: f32,
    my: f32,
    scale: f32,
    width: f32,
    height: f32,
) -> Option<(f32, f32)> {
    let s = 1.0 / scale as f32;
    let x = mx * s;
    let y = my * s;
    let window_width = width * s;
    let window_height = height * s;

    match mode {
        MouseMode::Pass => Some((x, y)),
        MouseMode::Clamp => Some((
            clamp(x, 0.0, window_width - 1.0),
            clamp(y, 0.0, window_height - 1.0),
        )),
        MouseMode::Discard => {
            if x < 0.0 || y < 0.0 || x >= window_width || y >= window_height {
                None
            } else {
                Some((x, y))
            }
        }
    }
}
