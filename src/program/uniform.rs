use std::collections::HashMap;

pub struct Constants {
    pub t: f32,
    pub resolution: [f32; 2],
    pub aspect: f32,
    pub mpos: [f32; 2],
    pub mclick: [f32; 2],
}

impl Constants {
    pub const SIZE: u32 = 40;
}

impl Default for Constants {
    fn default() -> Self {
        Self {
            t: 0.0,
            resolution: [0.0; 2],
            aspect: 1.0,
            mpos: [0.0; 2],
            mclick: [0.0; 2],
        }
    }
}

#[derive(Debug)]
pub enum Variable {
    Int(i32),
    Float(f32),
    Vec2([f32; 2]),
    Vec3([f32; 3]),
    Vec4([f32; 4]),
}

impl Variable {
    pub fn size(&self) -> u32 {
        match self {
            Variable::Int(_) => 4,
            Variable::Float(_) => 4,
            Variable::Vec2(_) => 8,
            Variable::Vec3(_) => 16,
            Variable::Vec4(_) => 16,
        }
    }

    pub fn bytes(&self) -> Vec<u8> {
        match self {
            Variable::Int(i) => i.to_le_bytes().to_vec(),
            Variable::Float(f) => f.to_le_bytes().to_vec(),
            Variable::Vec2(v) => [v[0].to_le_bytes(), v[1].to_le_bytes()].concat().to_vec(),
            Variable::Vec3(v) => [v[0].to_le_bytes(), v[1].to_le_bytes(), v[2].to_le_bytes()]
                .concat()
                .to_vec(),
            Variable::Vec4(v) => [
                v[0].to_le_bytes(),
                v[1].to_le_bytes(),
                v[2].to_le_bytes(),
                v[3].to_le_bytes(),
            ]
            .concat()
            .to_vec(),
        }
    }
}

pub struct UniformGroup {
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
    pub uniforms: HashMap<u32, Uniform>,
}

#[derive(Debug)]
pub struct Uniform {
    pub name: String,
    pub vars: Vec<(String, Variable)>,
    pub buffer: wgpu::Buffer,
}

impl Uniform {
    // Upload data in `vars` to the uniform buffer
    pub fn write(&self, queue: &wgpu::Queue) {
        let mut data: Vec<u8> = vec![];
        let mut offset = 0;
        for (_, var) in &self.vars {
            // Add padding until we're aligned to a multiple of the size
            while offset % var.size() != 0 {
                data.push(0);
                offset += 1;
            }

            // Write each byte of the variable's data
            for b in var.bytes() {
                data.push(b);
                offset += 1;
            }
        }

        queue.write_buffer(&self.buffer, 0, &data);
    }
}