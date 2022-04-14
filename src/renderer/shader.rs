use anyhow::Result;
use std::path::{Path, PathBuf};
use wgpu;

pub struct Shader {
    /// Path to the shader file
    path: PathBuf,
    
    /// Shader module to be passed to RenderPipeline
    module: wgpu::ShaderModule,
}

impl Shader {
    pub fn new<T: Clone>(device: &wgpu::Device, path: T) -> Result<Self> where T: AsRef<Path> {
        let source = std::fs::read_to_string(path.clone())?;
        let descriptor = wgpu::ShaderModuleDescriptor {
            label: Some("fractal_shader"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        };

        Ok(Self {
            path: path.as_ref().to_path_buf(),
            module: device.create_shader_module(&descriptor),
        })
    }

    pub fn get_module(&self) -> &wgpu::ShaderModule {
        &self.module
    }
}
