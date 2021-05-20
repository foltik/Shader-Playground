use imgui::*;

pub fn input_int(ui: &Ui, name: &str, v: &mut i32) {
    ui.input_int(&im_str!("{}", name), v)
        .build();
}

pub fn input_float(ui: &Ui, name: &str, v: &mut f32) {
    ui.input_float(&im_str!("{}", name), v)
        .build();

    let id = ui.push_id(&im_str!("{}.fr", name));
    Slider::new(&im_str!(""))
        .range(0.0..=1.0)
        .build(ui, v);
    id.pop(ui);
}

pub fn input_vec2(ui: &Ui, name: &str, v: &mut [f32; 2]) {
    ui.input_float2(&im_str!("{}", name), v)
        .build();

    let id = ui.push_id(&im_str!("{}.x", name));
    Slider::new(&im_str!("x"))
        .range(0.0..=1.0)
        .build(ui, &mut v[0]);
    id.pop(ui);

    let id = ui.push_id(&im_str!("{}.y", name));
    Slider::new(&im_str!("y"))
        .range(0.0..=1.0)
        .build(ui, &mut v[1]);
    id.pop(ui);
}

pub fn input_vec3(ui: &Ui, name: &str, v: &mut [f32; 3]) {
    ui.input_float3(&im_str!("{}", name), v)
        .build();

    let id = ui.push_id(&im_str!("{}.x", name));
    Slider::new(&im_str!("x"))
        .range(0.0..=1.0)
        .build(ui, &mut v[0]);
    id.pop(ui);

    let id = ui.push_id(&im_str!("{}.y", name));
    Slider::new(&im_str!("y"))
        .range(0.0..=1.0)
        .build(ui, &mut v[1]);
    id.pop(ui);

    let id = ui.push_id(&im_str!("{}.z", name));
    Slider::new(&im_str!("z"))
        .range(0.0..=1.0)
        .build(ui, &mut v[2]);
    id.pop(ui);
}

pub fn input_vec4(ui: &Ui, name: &str, v: &mut [f32; 4]) {
    ui.input_float4(&im_str!("{}", name), v)
        .build();

    let id = ui.push_id(&im_str!("{}.x", name));
    Slider::new(&im_str!("x"))
        .range(0.0..=1.0)
        .build(ui, &mut v[0]);
    id.pop(ui);

    let id = ui.push_id(&im_str!("{}.y", name));
    Slider::new(&im_str!("y"))
        .range(0.0..=1.0)
        .build(ui, &mut v[1]);
    id.pop(ui);

    let id = ui.push_id(&im_str!("{}.z", name));
    Slider::new(&im_str!("z"))
        .range(0.0..=1.0)
        .build(ui, &mut v[2]);
    id.pop(ui);

    let id = ui.push_id(&im_str!("{}.w", name));
    Slider::new(&im_str!("w"))
        .range(0.0..=1.0)
        .build(ui, &mut v[3]);
    id.pop(ui);
}