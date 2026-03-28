use bevy::{
    prelude::*,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
    sprite_render::{AlphaMode2d, Material2d},
};

/// 网格 shader uniform 数据
#[derive(Clone, Copy, ShaderType, Debug)]
pub struct GridUniforms {
    pub camera_pos: Vec2,
    pub view_size: Vec2,
    pub time: f32,
    pub tile_size: f32,
    pub _padding: Vec2,
}

impl Default for GridUniforms {
    fn default() -> Self {
        Self {
            camera_pos: Vec2::ZERO,
            view_size: Vec2::new(1920.0, 1080.0),
            time: 0.0,
            tile_size: 32.0,
            _padding: Vec2::ZERO,
        }
    }
}

/// Tron 风格发光网格地面材质
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone, Default)]
pub struct GridMaterial {
    #[uniform(0)]
    pub uniforms: GridUniforms,
}

impl Material2d for GridMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/grid.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Opaque
    }
}
