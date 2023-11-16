use std::{path::Path, ffi::{CString, NulError}, ptr, string::FromUtf8Error, collections::HashMap};

use glam::{Vec2, Vec3, Quat, Mat4};
use glow::{HasContext, ALWAYS, NativeTexture, NativeProgram};
use image::{ImageError, EncodableLayout, DynamicImage, Rgba};
use thiserror::Error;
use lazy_mut::lazy_mut;

lazy_mut! {
    pub static mut VIEW_MATRIX: Mat4 = Mat4::IDENTITY;
    pub static mut PROJECTION_MATRIX: Mat4 = Mat4::IDENTITY;
}

pub const DEFAULT_VERT: &'static str = r#"
#version 330
out vec2 texCoord;

in vec2 position;
in vec2 vertexTexCoord;

uniform mat4 mvp;
uniform float sort;

void main() {
    gl_Position = mvp * vec4(position, sort, 1.0);
    texCoord = vertexTexCoord;
}
"#;

pub const DEFAULT_FRAG: &'static str = r#"
#version 330
out vec4 FragColor;

in vec2 texCoord;

uniform sampler2D texture0;

void main() {
   FragColor = texture(texture0, texCoord);
}
"#;


#[derive(Clone)]
pub struct LilahTexture {
    pub id: NativeTexture,
    pub size: Vec2
}

impl LilahTexture {
    pub unsafe fn new(gl: &glow::Context) -> Self {
        let id: glow::NativeTexture = gl.create_texture().unwrap();
        Self { id, size: Vec2::ZERO }
    }

    pub unsafe fn load(&mut self, gl: &glow::Context, path: &Path) -> Result<(), ImageError> {
        self.bind(gl);

        let img: image::ImageBuffer<Rgba<u8>, Vec<u8>> = image::open(path)?.into_rgba8();
        gl.tex_image_2d(
            glow::TEXTURE_2D, 
            0, 
            glow::RGBA as i32, 
            img.width() as i32, 
            img.height() as i32, 
            0, 
            glow::RGBA, 
            glow::UNSIGNED_BYTE, 
            Some(&img.as_bytes())
        );
        self.size = Vec2::new(img.width() as f32, img.height() as f32);
        Ok(())
    }

    pub unsafe fn set_wrapping(&self, gl: &glow::Context, mode: i32) {
        self.bind(gl);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, mode);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, mode);
    }

    pub unsafe fn set_filtering(&self, gl: &glow::Context, mode: i32) {
        self.bind(gl);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, mode);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, mode);
    }

    pub unsafe fn bind(&self, gl: &glow::Context) {
        gl.bind_texture(glow::TEXTURE_2D, Some(self.id));
    }

    pub unsafe fn activate(&self, gl: &glow::Context, unit: u32) {
        gl.active_texture(unit);
        self.bind(gl);
    }
}

#[derive(Clone)]
pub struct VertexArray {
    pub id: glow::NativeVertexArray,
}

impl VertexArray {
    pub unsafe fn new(gl: &glow::Context) -> Self {
        let id: glow::NativeVertexArray = gl.create_vertex_array().unwrap();
        Self { id }
    }

    pub unsafe fn set_attribute<V: Sized>(
        &self,
        gl: &glow::Context,
        attrib_pos: u32,
        components: i32,
        offset: i32,
        precision: u32
    ) {
        self.bind(gl);
        gl.enable_vertex_attrib_array(attrib_pos);
        gl.vertex_attrib_pointer_f32(
            attrib_pos, 
            components, 
            precision, 
            false,
            std::mem::size_of::<V>() as i32, 
            offset
        );
    }

    pub unsafe fn bind(&self, gl: &glow::Context) {
        gl.bind_vertex_array(Some(self.id));
    }

    fn delete(&mut self, gl: &glow::Context) {
        unsafe {
            gl.delete_vertex_array(self.id);
        }
    }
}

#[macro_export]
macro_rules! set_attribute {
    ($gl:expr, $vbo:expr, $pos:tt, $t:ident :: $field:tt, $prec: expr) => {{
        let dummy = core::mem::MaybeUninit::<$t>::uninit();
        let dummy_ptr = dummy.as_ptr();
        let member_ptr = core::ptr::addr_of!((*dummy_ptr).$field);
        const fn size_of_raw<T>(_: *const T) -> usize {
            core::mem::size_of::<T>()
        }
        let member_offset = member_ptr as i32 - dummy_ptr as i32;
        $vbo.set_attribute::<$t>(
            $gl,
            $pos,
            (size_of_raw(member_ptr) / core::mem::size_of::<f32>()) as i32,
            member_offset,
            $prec
        )
    }};
}

#[derive(Debug, Error)]
pub enum ShaderError {
    #[error("Error while compiling shader: {0}")]
    CompilationError(String),
    #[error("Error while linking shaders: {0}")]
    LinkingError(String),
    #[error{"{0}"}]
    Utf8Error(#[from] FromUtf8Error),
    #[error{"{0}"}]
    NulError(#[from] NulError),
}

pub struct Shader {
    pub id: glow::NativeShader,
}

impl Shader {
    pub unsafe fn new(gl: &glow::Context, source_code: &str, shader_type: u32) -> Result<Self, ShaderError> {
        let shader = Self {
            id: gl.create_shader(shader_type).unwrap()
        };
        gl.shader_source(shader.id, source_code);
        gl.compile_shader(shader.id);

        let success = gl.get_shader_compile_status(shader.id);

        if success {
            Ok(shader)
        } else {
            let log = gl.get_shader_info_log(shader.id);
            Err(ShaderError::CompilationError(log))
        }
    }
}

pub struct ShaderProgram {
    pub id: glow::NativeProgram,
}

impl ShaderProgram {
    pub unsafe fn new(gl: &glow::Context, shaders: &[Shader]) -> Result<Self, ShaderError> {
        let program = Self {
            id: gl.create_program().unwrap()
        };

        for shader in shaders {
            gl.attach_shader(program.id, shader.id);
        }

        gl.link_program(program.id);

        let success = gl.get_program_link_status(program.id);

        if success {
            Ok(program)
        } else {
            let log = gl.get_program_info_log(program.id);
            Err(ShaderError::LinkingError(log))
        }
    }

    pub unsafe fn apply(&self, gl: &glow::Context) {
        gl.use_program(Some(self.id));
    }

    pub unsafe fn get_attrib_location(&self, gl: &glow::Context, attrib: &str) -> Option<u32> {
        gl.get_attrib_location(self.id, attrib)
    }

    pub unsafe fn set_int_uniform(&self, gl: &glow::Context, name: &str, value: i32) {
        self.apply(gl);
        gl.uniform_1_i32( gl.get_uniform_location(self.id, name).as_ref(), value)
    }
}

#[derive(Clone)]
pub struct Buffer {
    pub id: glow::NativeBuffer,
    target: u32,
}

impl Buffer {
    pub unsafe fn new(gl: &glow::Context, target: u32) -> Self {
        let id: glow::NativeBuffer = gl.create_buffer().unwrap();
        Self { id, target }
    }

    pub unsafe fn set_data<D>(&self, gl: &glow::Context, data: &[D], usage: u32) {
        self.bind(gl);
        let (_, data_bytes, _) = data.align_to::<u8>();
        let data_byte_buff = 
        core::slice::from_raw_parts(
            data_bytes.as_ptr() as *const _,
            data_bytes.len(),
        );

        gl.buffer_data_u8_slice(self.target, data_byte_buff, usage)
    }

    pub unsafe fn bind(&self, gl: &glow::Context) {
        gl.bind_buffer(self.target, Some(self.id));
    }

    fn delete(&mut self, gl: &glow::Context) {
        unsafe {
            gl.delete_buffer(self.id);
        }
    }
}

pub type Pos = [f32; 2];
pub type TextureCoords = [f32; 2];

#[repr(C, packed)]
pub struct Vertex(pub Pos, pub TextureCoords);

#[derive(Clone)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32
}

impl Color {
    pub const WHITE: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };

    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self {
            r,
            g,
            b,
            a
        }
    }
}

#[derive(Clone)]
pub struct Sprite {
    pub position: Vec2,
    /// size of sprite sheet
    base_size: (u32, u32),
    /// Start position on sprite sheet
    index_cut: (i32, i32),
    /// size of sprite cell
    size: (u32, u32),
    /// Current position on sprite sheet
    index: (i32, i32),
    /// Texture file name
    pub texture_id: String,

    pub tint: Color,

    pub sort: u32,
    
    vertex_buffer: Option<Buffer>,
    vertex_array: Option<VertexArray>
}

impl Sprite {
    #[rustfmt::skip]
    const DEF_VERTICES: [Vertex; 4] =  [
        Vertex([-0.5, -0.5],  [0.0, 1.0]),
        Vertex([ 0.5, -0.5],  [1.0, 1.0]),
        Vertex([ 0.5,  0.5],  [1.0, 0.0]),
        Vertex([-0.5,  0.5],  [0.0, 0.0]),
    ];

    #[rustfmt::skip]
    const DEF_INDICES: [i32; 6] = [
        0, 1, 2,
        2, 3, 0
    ];

    pub fn new(t_id: &str) -> Self {
        Self {
            position: Vec2::ZERO,
            size: (1, 1),
            base_size: (1, 1),
            index_cut: (0, 0),
            index: (0,0),
            texture_id: t_id.to_string(),
            vertex_array: None,
            vertex_buffer: None,
            tint: Color::WHITE,
            sort: 0
        }
    }

    pub fn load(&mut self, gl: &glow::Context, program: &ShaderProgram, textures: &HashMap<String, LilahTexture>) {
        let (vao , vbo) = unsafe {
            let vao = VertexArray::new(gl);
            vao.bind(gl);

            let vbo = Buffer::new(gl, glow::ARRAY_BUFFER);
            vbo.set_data(gl, &Sprite::DEF_VERTICES, glow::STATIC_DRAW);

            let ibo = Buffer::new(gl, glow::ELEMENT_ARRAY_BUFFER);
            ibo.set_data(gl, &Sprite::DEF_INDICES, glow::DYNAMIC_DRAW);

            let pos_attrib = program.get_attrib_location(gl, "position").unwrap();
            set_attribute!(gl, vao, pos_attrib, Vertex::0, glow::FLOAT);
            let color_attrib = program.get_attrib_location(gl, "vertexTexCoord").unwrap();
            set_attribute!(gl, vao, color_attrib, Vertex::1, glow::FLOAT);

            (vao, vbo)
        };

        self.vertex_array = Some(vao);
        self.vertex_buffer = Some(vbo);

        if let Some(t) = textures.get(&self.texture_id) {
            self.base_size = (
                t.size.x as u32,
                t.size.y as u32,
            );
        }
    }

    pub fn get_size(&self) -> (u32, u32) {
        (
            self.base_size.0/self.size.0,
            self.base_size.1/self.size.1,
        )
    }

    pub fn cut_sprite_sheet(&mut self, ind: i32, ind2: i32, col: u32, row: u32) {
        self.size = (col,row);
        self.index_cut = (ind, ind2);
        self.index = (0, 0);
    }

    pub fn anim_sprite_sheet(&mut self, gl: &glow::Context, program: &ShaderProgram, ind: i32, ind2: i32) {
        self.index = (ind*self.get_size().0 as i32, ind2*self.get_size().1 as i32);

        let zero = ((ind as f32 + 0.5) /self.size.0 as f32, (ind2 as f32 + 0.5)/self.size.1 as f32);
        let one = (zero.0+(self.size.0 as f32/self.base_size.0 as f32), zero.1+(self.size.1 as f32/self.base_size.1 as f32));

        let mut new_verts = Sprite::DEF_VERTICES;
        new_verts[0].1 = [zero.0, one.1];
        new_verts[1].1 = [one.0, one.1];
        new_verts[2].1 = [one.0, zero.1];
        new_verts[3].1 = [zero.0, zero.1];

        unsafe { 
            self.vertex_array.as_ref().unwrap().bind(gl);
            self.vertex_buffer.as_mut().unwrap().set_data(gl, &new_verts, glow::DYNAMIC_DRAW);
        }
    }

    pub fn draw(&self, gl: &glow::Context, program: &ShaderProgram, textures: &HashMap<String, LilahTexture>) {
        let model = 
        Mat4::IDENTITY * 
        Mat4::from_scale_rotation_translation( 
            Vec3::new(self.get_size().0 as f32, self.get_size().1 as f32, 1.0),
            Quat::from_rotation_z(0.0), 
            Vec3::new(self.position.x + (self.get_size().0/2) as f32, self.position.y - (self.get_size().1/2) as f32, 0.0)
        );

        let view = unsafe { *crate::renderer::VIEW_MATRIX };
        let projection = unsafe { *crate::renderer::PROJECTION_MATRIX };

        let mvp = projection * view * model;
        
        unsafe {
            textures[&self.texture_id].activate(gl, glow::TEXTURE0);

            program.apply(gl);
            
            self.vertex_array.as_ref().unwrap().bind(gl);
            
            let mat_attr = gl.get_uniform_location(program.id, "mvp").unwrap();
            gl.uniform_matrix_4_f32_slice(Some(&mat_attr), false,  &mvp.to_cols_array());

            let sort_attr = gl.get_uniform_location(program.id, "sort").unwrap();
            gl.uniform_1_f32(Some(&sort_attr), self.sort as f32);
            
            gl.draw_elements(glow::TRIANGLES, 6, glow::UNSIGNED_INT, 0);
        }
    }
}
