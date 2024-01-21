use imgui::*;

pub fn input_int(ui: &Ui, name: &str, v: &mut i32) {
    ui.input_int(name.to_string(), v)
        .build();
}

pub fn input_float(ui: &Ui, name: &str, v: &mut f32) {
    ui.input_float(name.to_string(), v)
        .build();

    let id = ui.push_id(format!("{}.fr", name));
    ui.slider("", 0.0, 1.0, v);
    id.pop();
}

pub fn input_vec2(ui: &Ui, name: &str, v: &mut [f32; 2]) {
    ui.input_float2(name.to_string(), v)
        .build();

    let id = ui.push_id(format!("{}.x", name));
    ui.slider("x", 0.0, 1.0, &mut v[0]);
    id.pop();

    let id = ui.push_id(format!("{}.y", name));
    ui.slider("y", 0.0, 1.0, &mut v[1]);
    id.pop();
}

pub fn input_vec3(ui: &Ui, name: &str, v: &mut [f32; 3]) {
    ui.input_float3(name.to_string(), v)
        .build();

    let id = ui.push_id(format!("{}.x", name));
    ui.slider("x", 0.0, 1.0, &mut v[0]);
    id.pop();

    let id = ui.push_id(format!("{}.y", name));
    ui.slider("y", 0.0, 1.0, &mut v[1]);
    id.pop();

    let id = ui.push_id(format!("{}.z", name));
    ui.slider("z", 0.0, 1.0, &mut v[2]);
    id.pop();
}

pub fn input_vec4(ui: &Ui, name: &str, v: &mut [f32; 4]) {
    ui.input_float4(name.to_string(), v)
        .build();

    let id = ui.push_id(format!("{}.x", name));
    ui.slider("x", 0.0, 1.0, &mut v[0]);
    id.pop();

    let id = ui.push_id(format!("{}.y", name));
    ui.slider("y", 0.0, 1.0, &mut v[1]);
    id.pop();

    let id = ui.push_id(format!("{}.z", name));
    ui.slider("z", 0.0, 1.0, &mut v[2]);
    id.pop();

    let id = ui.push_id(format!("{}.w", name));
    ui.slider("w", 0.0, 1.0, &mut v[2]);
    id.pop();
}
