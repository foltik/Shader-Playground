use spirv_reflect::types::{ReflectDescriptorType, ReflectTypeFlags};
use std::{collections::HashMap, sync::Arc};

pub mod compiler;
mod uniform;
pub mod watcher;

pub use uniform::{Constants, Uniform, UniformGroup, Variable};

pub struct Program {
    pub consts: Constants,

    pub pipeline: wgpu::RenderPipeline,
    pub uniform_groups: HashMap<u32, UniformGroup>,
}

// Container for a shader program and its variables / render state.
impl Program {
    pub fn new(
        device: Arc<wgpu::Device>,
        vertex: &wgpu::ShaderModule,
        fragment: wgpu::ShaderModule,
        reflect: spirv_reflect::ShaderModule,
    ) -> Result<Self, String> {
        // Keep track of the layout and
        let mut uniform_groups: HashMap<u32, HashMap<u32, Uniform>> = HashMap::new();

        // Enumerate the bind groups and individual bindings in the shader
        for binding in reflect.enumerate_descriptor_bindings(Some("main")).unwrap() {

            // If the binding isn't a uniform buffer, error out early to
            // prevent locking up the driver when the shader tries to use it.
            if binding.descriptor_type != ReflectDescriptorType::UniformBuffer {
                return Err(format!(
                    "Shader binding {} in set {} has unsupported type",
                    binding.set, binding.binding
                ));
            }

            // Get the type description for this uniform's structure
            let ty = binding.type_description.as_ref().ok_or_else(|| {
                format!(
                    "Failed to read type description for uniform {}",
                    binding.name
                )
            })?;

            // Keep track of the total size of all variables
            let mut size = 0;
            let mut vars = vec![];

            for var in &ty.members {
                // Make sure every variable's type is supported
                let mut v = None;
                if var.type_flags.contains(ReflectTypeFlags::VECTOR)
                    && var.type_flags.contains(ReflectTypeFlags::FLOAT)
                {
                    v = match var.traits.numeric.vector.component_count {
                        2 => Some(Variable::Vec2([1.0; 2])),
                        3 => Some(Variable::Vec3([1.0; 3])),
                        4 => Some(Variable::Vec4([1.0; 4])),
                        _ => None,
                    }
                } else if var.type_flags == ReflectTypeFlags::FLOAT {
                    v = match var.traits.numeric.scalar.width {
                        32 => Some(Variable::Float(1.0)),
                        _ => None,
                    }
                } else if var.type_flags == ReflectTypeFlags::INT {
                    v = match var.traits.numeric.scalar.signedness {
                        1 => Some(Variable::Int(1)),
                        _ => None,
                    }
                }
                let v = v.ok_or(format!(
                    "Variable \"{}\" in uniform \"{}\" has unsupported type",
                    &var.struct_member_name, &ty.type_name
                ))?;

                // Update, size, and align to a muliple of the size
                size += v.size();
                while size % v.size() != 0 {
                    size += 1;
                }

                vars.push((var.struct_member_name.clone(), v));
            }

            // Allocate a uniform buffer of the correct size
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: size as u64,
                usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
                mapped_at_creation: false,
            });

            let group = uniform_groups
                .entry(binding.set)
                .or_insert_with(HashMap::new);

            // Add the uniform to the bind group
            group.insert(
                binding.binding,
                Uniform {
                    name: ty.type_name.clone(),
                    vars,
                    buffer,
                },
            );
        }

        // Create bind groups for each uniform group
        let uniform_groups = uniform_groups
            .into_iter()
            .map(|(i, uniforms)| {
                let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: None,
                    entries: &uniforms
                        .iter()
                        .map(|(j, _)| wgpu::BindGroupLayoutEntry {
                            binding: *j,
                            visibility: wgpu::ShaderStage::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                min_binding_size: None,
                                has_dynamic_offset: false,
                            },
                            count: None,
                        })
                        .collect::<Vec<_>>(),
                });

                let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: &bind_group_layout,
                    entries: &uniforms
                        .iter()
                        .map(|(j, u)| wgpu::BindGroupEntry {
                            binding: *j,
                            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                                buffer: &u.buffer,
                                offset: 0,
                                size: None,
                            }),
                        })
                        .collect::<Vec<_>>(),
                });

                (i, UniformGroup {
                    bind_group_layout,
                    bind_group,
                    uniforms,
                })
            })
            .collect::<HashMap<_, _>>();

        // Create the pipeline
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &uniform_groups
                .iter()
                .map(|(_, g)| &g.bind_group_layout)
                .collect::<Vec<_>>(),
            push_constant_ranges: &[wgpu::PushConstantRange {
                stages: wgpu::ShaderStage::all(),
                range: 0..Constants::SIZE,
            }],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &vertex,
                entry_point: "main",
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            fragment: Some(wgpu::FragmentState {
                module: &fragment,
                entry_point: "main",
                targets: &[crate::render::FORMAT.into()],
            }),
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
        });

        Ok(Self {
            consts: Default::default(),

            pipeline,
            uniform_groups,
        })
    }

    // Initialize variables with the same name to the values from another program.
    // TODO: There's probably a cleaner way to do this...
    pub fn initialize(&mut self, other: &Program) {
        for (i, group) in &mut self.uniform_groups {
            if let Some(ogroup) = other.uniform_groups.get(i) {
                for (j, uniform) in &mut group.uniforms {
                    if let Some(ouniform) = ogroup.uniforms.get(j) {
                        for (name, v) in &mut uniform.vars {
                            for (oname, ov) in &ouniform.vars {
                                if name == oname {
                                    // Try to pick sane defaults, ex. convert int to float if type changed.
                                    match v {
                                        Variable::Int(i) => match ov {
                                            Variable::Int(oi) => *i = *oi,
                                            Variable::Float(of) => *i = *of as i32,
                                            _ => {}
                                        },
                                        Variable::Float(f) => match ov {
                                            Variable::Float(of) => *f = *of,
                                            Variable::Int(oi) => *f = *oi as f32,
                                            _ => {}
                                        }
                                        Variable::Vec2(a) => match ov {
                                            Variable::Vec2(oa) => a.copy_from_slice(oa),
                                            Variable::Vec3(oa) => a.copy_from_slice(&oa[..2]),
                                            Variable::Vec4(oa) => a.copy_from_slice(&oa[..2]),
                                            _ => {}
                                        },
                                        Variable::Vec3(a) => match ov {
                                            Variable::Vec2(oa) => a.copy_from_slice(&[oa[0], oa[1], 0.0]),
                                            Variable::Vec3(oa) => a.copy_from_slice(oa),
                                            Variable::Vec4(oa) => a.copy_from_slice(&oa[..3]),
                                            _ => {}
                                        },
                                        Variable::Vec4(a) => match ov {
                                            Variable::Vec2(oa) => a.copy_from_slice(&[oa[0], oa[1], 0.0, 0.0]),
                                            Variable::Vec3(oa) => a.copy_from_slice(&[oa[0], oa[1], oa[2], 0.0]),
                                            Variable::Vec4(oa) => a.copy_from_slice(oa),
                                            _ => {}
                                        },
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

    }
}
