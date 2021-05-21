# Shader Playground

A shader editor inspired by ShaderToy featuring live reloading and automatic GUI generation for uniform variables.

Try the demo with `cargo run demo.glsl`.

![image](https://user-images.githubusercontent.com/5026551/119198566-102b1480-ba79-11eb-9518-692a75b2563f.png)

## Details

Once opened, writing to the file will trigger a recompile of the shader.

Several built-in push constants are available, see `demo.glsl` for the full list.

Any uniform structures present and used in the shader will generate an editable GUI as long as the field types are all either `int`, `float`, `vec2`, `vec3`, or `vec4`.

## Compatibility

As of now I've only tested on Linux with Vulkan, so other platforms may not work. Pull requests welcome.
