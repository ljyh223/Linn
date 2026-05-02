//! Bicubic Hermite Patch Mesh renderer
//!
//! Ported from AMLL's mesh-renderer/index.ts.
//! Uses bicubic Hermite surface mesh gradient for album art background effect.

use super::control_points::{ControlPointConf, ControlPointPreset};
use super::shaders;
use glow::HasContext;

const SUBDIVISION: usize = 50;

#[derive(Clone, Debug)]
pub struct MeshConfig {
    pub flow_speed: f32,       // 流动速度，默认 1.0
    pub brightness: f32,       // 最终亮度，默认 0.75
    pub saturation: f32,       // 饱和度，默认 3.0
    pub contrast_1: f32,       // 第一次对比度压缩，默认 0.4
    pub contrast_2: f32,       // 第二次对比度放大，默认 1.7
    pub blur_radius: u32,      // 模糊半径，默认 2
    pub blur_iterations: u32,  // 模糊迭代次数，默认 4
}

impl Default for MeshConfig {
    fn default() -> Self {
        Self {
            flow_speed: 1.0,
            brightness: 0.75,
            saturation: 3.0,
            contrast_1: 0.4,
            contrast_2: 1.7,
            blur_radius: 2,
            blur_iterations: 4,
        }
    }
}

/// Hermite basis matrix H (row-major)
const H: [[f64; 4]; 4] = [
    [2.0, -3.0, 0.0, 1.0],
    [-2.0, 3.0, 0.0, 0.0],
    [1.0, -2.0, 1.0, 0.0],
    [1.0, -1.0, 0.0, 0.0],
];

fn mul_mat4(a: &[[f64; 4]; 4], b: &[[f64; 4]; 4]) -> [[f64; 4]; 4] {
    let mut r = [[0.0f64; 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            for k in 0..4 {
                r[i][j] += a[i][k] * b[k][j];
            }
        }
    }
    r
}

fn transpose_mat4(m: &[[f64; 4]; 4]) -> [[f64; 4]; 4] {
    let mut r = [[0.0f64; 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            r[i][j] = m[j][i];
        }
    }
    r
}

fn mesh_coefficients(
    p00: &ControlPointConf,
    p01: &ControlPointConf,
    p10: &ControlPointConf,
    p11: &ControlPointConf,
    u_power: f64,
    v_power: f64,
    axis: usize,
) -> [[f64; 4]; 4] {
    let loc = |cp: &ControlPointConf| -> f64 { if axis == 0 { cp.x } else { cp.y } };
    let u_tan = |cp: &ControlPointConf| -> f64 {
        let ur_rad = cp.ur * std::f64::consts::PI / 180.0;
        if axis == 0 {
            ur_rad.cos() * u_power * cp.up
        } else {
            ur_rad.sin() * u_power * cp.up
        }
    };
    let v_tan = |cp: &ControlPointConf| -> f64 {
        let vr_rad = cp.vr * std::f64::consts::PI / 180.0;
        if axis == 0 {
            -vr_rad.sin() * v_power * cp.vp
        } else {
            vr_rad.cos() * v_power * cp.vp
        }
    };

    let m = [
        [loc(p00), loc(p10), u_tan(p00), u_tan(p10)],
        [loc(p01), loc(p11), u_tan(p01), u_tan(p11)],
        [v_tan(p00), v_tan(p10), 0.0, 0.0],
        [v_tan(p01), v_tan(p11), 0.0, 0.0],
    ];

    let ht = transpose_mat4(&H);
    let mt = transpose_mat4(&m);
    mul_mat4(&mul_mat4(&ht, &mt), &H)
}

struct BHPMesh {
    grid_w: usize,
    grid_h: usize,
    control_points: Vec<ControlPointConf>,
    vertices: Vec<f32>,
    indices: Vec<u32>,
    album_colors: Vec<[f32; 3]>,
}

impl BHPMesh {
    fn new(preset: &ControlPointPreset) -> Self {
        let gw = preset.width;
        let gh = preset.height;
        let mut m = Self {
            grid_w: gw,
            grid_h: gh,
            control_points: preset.conf.clone(),
            vertices: Vec::new(),
            indices: Vec::new(),
            album_colors: Vec::new(),
        };
        m.subdivide();
        m
    }

    fn get_cp(&self, gx: usize, gy: usize) -> &ControlPointConf {
        &self.control_points[gy * self.grid_w + gx]
    }

    fn subdivide(&mut self) {
        self.vertices.clear();
        self.indices.clear();

        let sub = SUBDIVISION;
        let gw = self.grid_w;
        let gh = self.grid_h;
        let u_power = 2.0 / (gw as f64 - 1.0);
        let v_power = 2.0 / (gh as f64 - 1.0);
        let patches_x = gw - 1;
        let patches_y = gh - 1;
        let has_colors = !self.album_colors.is_empty();

        let verts_x = patches_x * sub + 1;
        let verts_y = patches_y * sub + 1;

        for vy in 0..verts_y {
            for vx in 0..verts_x {
                let (patch_x, lu) = if vx == verts_x - 1 {
                    (patches_x - 1, sub)
                } else {
                    (vx / sub, vx % sub)
                };
                let (patch_y, lv) = if vy == verts_y - 1 {
                    (patches_y - 1, sub)
                } else {
                    (vy / sub, vy % sub)
                };

                let u = lu as f64 / sub as f64;
                let v = lv as f64 / sub as f64;
                let uv_pow = [u*u*u, u*u, u, 1.0];
                let vv_pow = [v*v*v, v*v, v, 1.0];

                let p00 = self.get_cp(patch_x, patch_y);
                let p10 = self.get_cp(patch_x + 1, patch_y);
                let p01 = self.get_cp(patch_x, patch_y + 1);
                let p11 = self.get_cp(patch_x + 1, patch_y + 1);

                let hg_x = mesh_coefficients(p00, p01, p10, p11, u_power, v_power, 0);
                let hg_y = mesh_coefficients(p00, p01, p10, p11, u_power, v_power, 1);

                let eval = |hg: &[[f64; 4]; 4]| -> f64 {
                    let mut r = 0.0;
                    for i in 0..4 {
                        for j in 0..4 {
                            r += uv_pow[i] * hg[i][j] * vv_pow[j];
                        }
                    }
                    r
                };

                let x = eval(&hg_x) as f32;
                let y = eval(&hg_y) as f32;

                let tex_u = vx as f32 / (verts_x - 1) as f32;
                let tex_v = 1.0 - vy as f32 / (verts_y - 1) as f32;

                let r = 1.0f32;
                let g = 1.0f32;
                let b = 1.0f32;

                self.vertices.extend_from_slice(&[x, y, tex_u, tex_v, r, g, b]);
            }
        }

        for vy in 0..(verts_y - 1) {
            for vx in 0..(verts_x - 1) {
                let tl = (vy * verts_x + vx) as u32;
                let tr = tl + 1;
                let bl = tl + verts_x as u32;
                let br = bl + 1;
                self.indices.extend_from_slice(&[tl, bl, tr, tr, bl, br]);
            }
        }
    }

    fn set_album_colors(&mut self, colors: Vec<[f32; 3]>) {
        self.album_colors = colors;
        self.subdivide();
    }
}

/// A combined structure holding a mesh and its corresponding GL state (VAO/VBO/Tex).
/// Doing this enables smooth cross-fading between meshes without reloading buffers mid-frame.
struct RenderMesh {
    mesh: BHPMesh,
    vao: glow::VertexArray,
    vbo: glow::Buffer,
    ebo: glow::Buffer,
    album_tex: glow::Texture,
}

impl RenderMesh {
    unsafe fn new(gl: &glow::Context, mesh: BHPMesh, tex: glow::Texture) -> Self {
        let vao = gl.create_vertex_array().unwrap();
        let vbo = gl.create_buffer().unwrap();
        let ebo = gl.create_buffer().unwrap();
        
        gl.bind_vertex_array(Some(vao));
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
        gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(ebo));
        
        let stride = 7 * 4i32;
        gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, stride, 0); // pos
        gl.enable_vertex_attrib_array(0);
        gl.vertex_attrib_pointer_f32(1, 2, glow::FLOAT, false, stride, 8); // a_texCoord
        gl.enable_vertex_attrib_array(1);
        gl.vertex_attrib_pointer_f32(2, 3, glow::FLOAT, false, stride, 16); // a_color
        gl.enable_vertex_attrib_array(2);
        
        // Static upload: vertices are dynamically displaced via Vertex Shader now!
        gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, f32_bytes(&mesh.vertices), glow::STATIC_DRAW);
        gl.buffer_data_u8_slice(glow::ELEMENT_ARRAY_BUFFER, u32_bytes(&mesh.indices), glow::STATIC_DRAW);
        gl.bind_vertex_array(None);
        
        Self { mesh, vao, vbo, ebo, album_tex: tex }
    }

    unsafe fn destroy(&self, gl: &glow::Context) {
        gl.delete_vertex_array(self.vao);
        gl.delete_buffer(self.vbo);
        gl.delete_buffer(self.ebo);
        gl.delete_texture(self.album_tex);
    }
}

pub struct MeshGradientRenderer {
    program: Option<glow::Program>,
    quad_program: Option<glow::Program>,
    
    fbo: Option<glow::Framebuffer>,
    fbo_tex: Option<glow::Texture>,
    quad_vao: Option<glow::VertexArray>,
    quad_vbo: Option<glow::Buffer>,
    
    current_mesh: Option<RenderMesh>,
    old_mesh: Option<RenderMesh>,

    pub config: MeshConfig, 
    
    time: f32,
    trans_alpha: f32,
    initialized: bool,
    w: i32,
    h: i32,
    draw_count: u32,
}

impl MeshGradientRenderer {
    pub fn new() -> Self {
        Self {
            program: None,
            quad_program: None,
            fbo: None,
            fbo_tex: None,
            quad_vao: None,
            quad_vbo: None,
            current_mesh: None,
            old_mesh: None,
            config: MeshConfig::default(),
            time: 0.0,
            trans_alpha: 1.0,
            initialized: false,
            w: 0,
            h: 0,
            draw_count: 0,
        }
    }

    pub fn initialize(&mut self, gl: &glow::Context) {
        unsafe {
            self.program = Some(create_program(
                gl,
                shaders::MESH_VERTEX_SHADER,
                shaders::MESH_FRAGMENT_SHADER,
            ));
            self.quad_program = Some(create_program(
                gl,
                shaders::QUAD_VERTEX_SHADER,
                shaders::QUAD_FRAGMENT_SHADER,
            ));

            let qvao = gl.create_vertex_array().unwrap();
            let qvbo = gl.create_buffer().unwrap();
            gl.bind_vertex_array(Some(qvao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(qvbo));
            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 16, 0);
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(1, 2, glow::FLOAT, false, 16, 8);
            gl.enable_vertex_attrib_array(1);
            gl.bind_vertex_array(None);
            self.quad_vao = Some(qvao);
            self.quad_vbo = Some(qvbo);

            self.create_fbo(gl, 800, 600);
            
            let presets = super::control_points::get_all_presets();
            let preset = &presets[0];
            let mesh = BHPMesh::new(preset);
            let tex = create_dummy_texture(gl);
            
            log::info!(
                "MeshGradientRenderer: initialize with preset {}x{}, {} control points",
                preset.width, preset.height, preset.conf.len()
            );
            
            self.current_mesh = Some(RenderMesh::new(gl, mesh, tex));
        }
        self.initialized = true;
    }

    unsafe fn create_fbo(&mut self, gl: &glow::Context, w: i32, h: i32) {
        unsafe {
            let prev_fbo = {
                use std::num::NonZeroU32;
                let raw = gl.get_parameter_i32(glow::FRAMEBUFFER_BINDING) as u32;
                NonZeroU32::new(raw).map(glow::NativeFramebuffer)
            };
            if let Some(f) = self.fbo {
                gl.delete_framebuffer(f);
            }
            if let Some(t) = self.fbo_tex {
                gl.delete_texture(t);
            }
            let fbo = gl.create_framebuffer().unwrap();
            let tex = gl.create_texture().unwrap();
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
            gl.bind_texture(glow::TEXTURE_2D, Some(tex));
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA8 as i32,
                w,
                h,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(None),
            );
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D,
                Some(tex),
                0,
            );
            let status = gl.check_framebuffer_status(glow::FRAMEBUFFER);
            if status != glow::FRAMEBUFFER_COMPLETE {
                log::error!("FBO incomplete: status={status:#x}");
            }
            gl.bind_framebuffer(glow::FRAMEBUFFER, prev_fbo);
            self.fbo = Some(fbo);
            self.fbo_tex = Some(tex);
            self.w = w;
            self.h = h;
        }
    }

    pub fn set_album(&mut self, gl: &glow::Context, data: &[u8], _img_w: i32, _img_h: i32) {
        if !self.initialized { return; }

        let image = image::load_from_memory(data)
            .map(|img| img.to_rgba8())
            .unwrap_or_else(|_| {
                let mut fb = image::RgbaImage::new(1, 1);
                fb.put_pixel(0, 0, image::Rgba([128, 128, 128, 255]));
                fb
            });
        let (w, h) = image.dimensions();

        let small = image::imageops::resize(&image, 32, 32, image::imageops::FilterType::Triangle);
        let (sw, sh) = small.dimensions();

        let cfg = &self.config;

        let mut processed = small.clone();
        for px in processed.pixels_mut() {
            let r = px[0] as f32;
            let g = px[1] as f32;
            let b = px[2] as f32;
            
            // contrast_1
            let r = (r - 128.0) * cfg.contrast_1 + 128.0;
            let g = (g - 128.0) * cfg.contrast_1 + 128.0;
            let b = (b - 128.0) * cfg.contrast_1 + 128.0;
            
            // saturate
            let gray = 0.3 * r + 0.59 * g + 0.11 * b;
            let r = gray * (1.0 - cfg.saturation) + r * cfg.saturation;
            let g = gray * (1.0 - cfg.saturation) + g * cfg.saturation;
            let b = gray * (1.0 - cfg.saturation) + b * cfg.saturation;
            
            // contrast_2
            let r = (r - 128.0) * cfg.contrast_2 + 128.0;
            let g = (g - 128.0) * cfg.contrast_2 + 128.0;
            let b = (b - 128.0) * cfg.contrast_2 + 128.0;
            
            // brightness
            let r = r * cfg.brightness;
            let g = g * cfg.brightness;
            let b = b * cfg.brightness;
            
            px[0] = r.clamp(0.0, 255.0) as u8;
            px[1] = g.clamp(0.0, 255.0) as u8;
            px[2] = b.clamp(0.0, 255.0) as u8;
        }
        let blurred = blur_image(&processed, cfg.blur_radius, cfg.blur_iterations);
        let tex_data = blurred.as_raw();

        let tex = unsafe {
            let t = gl.create_texture().unwrap();
            gl.bind_texture(glow::TEXTURE_2D, Some(t));
            gl.tex_image_2d(
                glow::TEXTURE_2D, 0, glow::RGBA8 as i32,
                sw as i32, sh as i32, 0,
                glow::RGBA, glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(tex_data)),
            );
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
            
            // 【极其关键的修复】：使用 MIRRORED_REPEAT 防止 UV 旋转时出现边缘单色拉伸
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::MIRRORED_REPEAT as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::MIRRORED_REPEAT as i32);
            t
        };
        let presets = super::control_points::get_all_presets();
        let pi = (self.time as usize) % presets.len();
        let preset = &presets[pi];
        let gw = preset.width;
        let gh = preset.height;

        let mut colors = Vec::new();
        for gy in 0..gh {
            for gx in 0..gw {
                let sx = (gx as f32 / (gw as f32 - 1.0) * (w as f32 - 1.0)).round() as u32;
                let sy = (gy as f32 / (gh as f32 - 1.0) * (h as f32 - 1.0)).round() as u32;
                let px = image.get_pixel(sx.min(w - 1), sy.min(h - 1));
                let r = px[0] as f32 / 255.0;
                let g = px[1] as f32 / 255.0;
                let b = px[2] as f32 / 255.0;
                let r1 = (r - 0.5) * 0.4 + 0.5;
                let g1 = (g - 0.5) * 0.4 + 0.5;
                let b1 = (b - 0.5) * 0.4 + 0.5;
                let gray = 0.3 * r1 + 0.59 * g1 + 0.11 * b1;
                let r2 = gray * -2.0 + r1 * 3.0;
                let g2 = gray * -2.0 + g1 * 3.0;
                let b2 = gray * -2.0 + b1 * 3.0;
                let r3 = (r2 - 0.5) * 1.7 + 0.5;
                let g3 = (g2 - 0.5) * 1.7 + 0.5;
                let b3 = (b2 - 0.5) * 1.7 + 0.5;
                colors.push([
                    r3 * 0.75,
                    g3 * 0.75,
                    b3 * 0.75,
                ]);
            }
        }

        let mut mesh = BHPMesh::new(preset);
        mesh.set_album_colors(colors);
        
        // 跨专辑平滑过渡逻辑
        if let Some(old) = self.old_mesh.take() {
            unsafe { old.destroy(gl); }
        }
        if let Some(curr) = self.current_mesh.take() {
            self.old_mesh = Some(curr);
        }
        self.current_mesh = unsafe { Some(RenderMesh::new(gl, mesh, tex)) };
        self.trans_alpha = 0.0;
    }

    pub fn draw(&mut self, gl: &glow::Context, ww: i32, wh: i32) {
        if !self.initialized {
            return;
        }
        if ww <= 0 || wh <= 0 {
            return;
        }
        self.time += 0.0016 * self.config.flow_speed; 

        if self.trans_alpha < 1.0 {
            self.trans_alpha = (self.trans_alpha + 0.02).min(1.0);
            if self.trans_alpha >= 1.0 {
                // 过渡结束，清理旧 Mesh 资源
                if let Some(old) = self.old_mesh.take() {
                    unsafe { old.destroy(gl); }
                }
            }
        }
        
        if ww != self.w || wh != self.h {
            unsafe {
                self.create_fbo(gl, ww, wh);
            }
        }
        
        let draw_count = self.draw_count;
        self.draw_count += 1;
        if draw_count <= 3 {
            if let Some(curr) = &self.current_mesh {
                let mesh = &curr.mesh;
                eprintln!(
                    "MeshGradientRenderer::draw #{draw_count}: size={}x{}, alpha={}, has_album={}, has_mesh=true",
                    ww, wh, self.trans_alpha, true
                );
                eprintln!(
                    "  mesh: {} verts, {} indices, grid {}x{}, colors={}",
                    mesh.vertices.len() / 7, mesh.indices.len(),
                    mesh.grid_w, mesh.grid_h, mesh.album_colors.len()
                );
            }
        }
        
        unsafe {
            let default_fb = {
                use std::num::NonZeroU32;
                let raw = gl.get_parameter_i32(glow::FRAMEBUFFER_BINDING) as u32;
                NonZeroU32::new(raw).map(glow::NativeFramebuffer)
            };

            // Pass 1: 渲染 mesh 到 FBO，支持新旧 mesh 交叉淡入淡出
            gl.bind_framebuffer(glow::FRAMEBUFFER, self.fbo);
            gl.viewport(0, 0, ww, wh);
            gl.enable(glow::BLEND);
            gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
            gl.clear_color(0.0, 0.0, 0.0, 0.0);
            gl.clear(glow::COLOR_BUFFER_BIT);
            
            if let Some(prog) = self.program {
                gl.use_program(Some(prog));
                set_f(gl, prog, "u_time", self.time);
                set_f(gl, prog, "u_volume", 0.0);
                set_f(gl, prog, "u_aspectRatio", ww as f32 / wh as f32);

                // 绘制旧 Mesh（做底）
                if let Some(old) = &self.old_mesh {
                    set_f(gl, prog, "u_alpha", 1.0);
                    set_vec2(gl, prog, "u_grid_size", (old.mesh.grid_w - 1) as f32, (old.mesh.grid_h - 1) as f32);
                    gl.active_texture(glow::TEXTURE0);
                    gl.bind_texture(glow::TEXTURE_2D, Some(old.album_tex));
                    set_i(gl, prog, "u_texture", 0);
                    set_i(gl, prog, "u_has_texture", 1);

                    gl.bind_vertex_array(Some(old.vao));
                    gl.draw_elements(glow::TRIANGLES, old.mesh.indices.len() as i32, glow::UNSIGNED_INT, 0);
                }

                // 绘制新 Mesh（淡入）
                if let Some(curr) = &self.current_mesh {
                    let current_alpha = if self.old_mesh.is_some() { self.trans_alpha } else { 1.0 };
                    set_f(gl, prog, "u_alpha", current_alpha);
                    set_vec2(gl, prog, "u_grid_size", (curr.mesh.grid_w - 1) as f32, (curr.mesh.grid_h - 1) as f32);
                    gl.active_texture(glow::TEXTURE0);
                    gl.bind_texture(glow::TEXTURE_2D, Some(curr.album_tex));
                    set_i(gl, prog, "u_texture", 0);
                    set_i(gl, prog, "u_has_texture", 1);

                    gl.bind_vertex_array(Some(curr.vao));
                    gl.draw_elements(glow::TRIANGLES, curr.mesh.indices.len() as i32, glow::UNSIGNED_INT, 0);
                }
                gl.bind_vertex_array(None);
            }

            // Pass 2: 合成 FBO 到 GTK 默认 Framebuffer
            gl.bind_framebuffer(glow::FRAMEBUFFER, default_fb);
            gl.viewport(0, 0, ww, wh);
            gl.enable(glow::BLEND);
            gl.blend_func_separate(
                glow::SRC_ALPHA,
                glow::ONE_MINUS_SRC_ALPHA,
                glow::ONE,
                glow::ONE_MINUS_SRC_ALPHA,
            );
            
            if let (Some(qp), Some(qvao), Some(ft)) =
                (self.quad_program, self.quad_vao, self.fbo_tex)
            {
                gl.use_program(Some(qp));
                // 由于 FBO 内部已经完成了交叉淡入，这里的整体透明度给 1.0 即可
                set_f(gl, qp, "u_alpha", 1.0);
                gl.active_texture(glow::TEXTURE0);
                gl.bind_texture(glow::TEXTURE_2D, Some(ft));
                set_i(gl, qp, "u_texture", 0);
                gl.bind_vertex_array(Some(qvao));
                
                let qv: [f32; 24] = [
                    -1.0, -1.0, 0.0, 0.0, 1.0, -1.0, 1.0, 0.0, -1.0, 1.0, 0.0, 1.0, -1.0, 1.0, 0.0,
                    1.0, 1.0, -1.0, 1.0, 0.0, 1.0, 1.0, 1.0, 1.0,
                ];
                gl.bind_buffer(glow::ARRAY_BUFFER, self.quad_vbo);
                gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, f32_bytes(&qv), glow::DYNAMIC_DRAW);
                gl.draw_arrays(glow::TRIANGLES, 0, 6);
                gl.bind_vertex_array(None);
            }
        }
    }

    pub fn cleanup(&mut self, gl: &glow::Context) {
        if !self.initialized {
            return;
        }
        unsafe {
            if let Some(p) = self.program { gl.delete_program(p); }
            if let Some(p) = self.quad_program { gl.delete_program(p); }
            if let Some(f) = self.fbo { gl.delete_framebuffer(f); }
            if let Some(t) = self.fbo_tex { gl.delete_texture(t); }
            if let Some(v) = self.quad_vao { gl.delete_vertex_array(v); }
            if let Some(v) = self.quad_vbo { gl.delete_buffer(v); }
            
            if let Some(m) = &self.current_mesh { m.destroy(gl); }
            if let Some(m) = &self.old_mesh { m.destroy(gl); }
            self.current_mesh = None;
            self.old_mesh = None;
        }
        self.initialized = false;
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
}

unsafe fn create_dummy_texture(gl: &glow::Context) -> glow::Texture {
    let tex = gl.create_texture().unwrap();
    gl.bind_texture(glow::TEXTURE_2D, Some(tex));
    gl.tex_image_2d(
        glow::TEXTURE_2D, 0, glow::RGBA8 as i32,
        1, 1, 0,
        glow::RGBA, glow::UNSIGNED_BYTE,
        glow::PixelUnpackData::Slice(Some(&[128, 128, 128, 255])),
    );
    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR as i32);
    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);
    tex
}

unsafe fn compile_shader(gl: &glow::Context, ty: u32, src: &str) -> glow::Shader {
    unsafe {
        let s = gl.create_shader(ty).unwrap();
        gl.shader_source(s, src);
        gl.compile_shader(s);
        if !gl.get_shader_compile_status(s) {
            panic!("Shader error: {}", gl.get_shader_info_log(s));
        }
        s
    }
}

unsafe fn create_program(gl: &glow::Context, vs: &str, fs: &str) -> glow::Program {
    unsafe {
        let v = compile_shader(gl, glow::VERTEX_SHADER, vs);
        let f = compile_shader(gl, glow::FRAGMENT_SHADER, fs);
        let p = gl.create_program().unwrap();
        gl.attach_shader(p, v);
        gl.attach_shader(p, f);
        gl.link_program(p);
        if !gl.get_program_link_status(p) {
            panic!("Link error: {}", gl.get_program_info_log(p));
        }
        gl.detach_shader(p, v);
        gl.detach_shader(p, f);
        gl.delete_shader(v);
        gl.delete_shader(f);
        p
    }
}

unsafe fn set_f(gl: &glow::Context, prog: glow::Program, name: &str, val: f32) {
    unsafe {
        gl.uniform_1_f32(gl.get_uniform_location(prog, name).as_ref(), val);
    }
}

unsafe fn set_vec2(gl: &glow::Context, prog: glow::Program, name: &str, x: f32, y: f32) {
    unsafe {
        gl.uniform_2_f32(gl.get_uniform_location(prog, name).as_ref(), x, y);
    }
}

unsafe fn set_i(gl: &glow::Context, prog: glow::Program, name: &str, val: i32) {
    unsafe {
        gl.uniform_1_i32(gl.get_uniform_location(prog, name).as_ref(), val);
    }
}

fn f32_bytes(d: &[f32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(d.as_ptr() as *const u8, d.len() * 4) }
}
fn u32_bytes(d: &[u32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(d.as_ptr() as *const u8, d.len() * 4) }
}

fn blur_image(img: &image::RgbaImage, radius: u32, iterations: u32) -> image::RgbaImage {
    let mut current = img.clone();
    for _ in 0..iterations {
        current = box_blur_h(&current, radius);
        current = box_blur_v(&current, radius);
    }
    current
}

fn box_blur_h(img: &image::RgbaImage, radius: u32) -> image::RgbaImage {
    let (w, h) = img.dimensions();
    let mut out = image::RgbaImage::new(w, h);
    let r = radius as i32;
    let diameter = (2 * r + 1) as f32;
    for y in 0..h {
        for x in 0..w {
            let mut sum = [0.0f32; 3];
            for dx in -r..=r {
                let sx = ((x as i32 + dx).clamp(0, w as i32 - 1)) as u32;
                let px = img.get_pixel(sx, y);
                sum[0] += px[0] as f32;
                sum[1] += px[1] as f32;
                sum[2] += px[2] as f32;
            }
            let px = img.get_pixel(x, y);
            out.put_pixel(
                x,
                y,
                image::Rgba([
                    (sum[0] / diameter).clamp(0.0, 255.0) as u8,
                    (sum[1] / diameter).clamp(0.0, 255.0) as u8,
                    (sum[2] / diameter).clamp(0.0, 255.0) as u8,
                    px[3],
                ]),
            );
        }
    }
    out
}

fn box_blur_v(img: &image::RgbaImage, radius: u32) -> image::RgbaImage {
    let (w, h) = img.dimensions();
    let mut out = image::RgbaImage::new(w, h);
    let r = radius as i32;
    let diameter = (2 * r + 1) as f32;
    for x in 0..w {
        for y in 0..h {
            let mut sum = [0.0f32; 3];
            for dy in -r..=r {
                let sy = ((y as i32 + dy).clamp(0, h as i32 - 1)) as u32;
                let px = img.get_pixel(x, sy);
                sum[0] += px[0] as f32;
                sum[1] += px[1] as f32;
                sum[2] += px[2] as f32;
            }
            let px = img.get_pixel(x, y);
            out.put_pixel(
                x,
                y,
                image::Rgba([
                    (sum[0] / diameter).clamp(0.0, 255.0) as u8,
                    (sum[1] / diameter).clamp(0.0, 255.0) as u8,
                    (sum[2] / diameter).clamp(0.0, 255.0) as u8,
                    px[3],
                ]),
            );
        }
    }
    out
}

fn sample_colors_bilinear(
    colors: &[[f32; 3]],
    gw: usize,
    gh: usize,
    u: f32,  // 0.0 ~ 1.0
    v: f32,  // 0.0 ~ 1.0
) -> (f32, f32, f32) {
    let x = (u * (gw - 1) as f32).clamp(0.0, (gw - 1) as f32);
    let y = (v * (gh - 1) as f32).clamp(0.0, (gh - 1) as f32);
    let x0 = x.floor() as usize;
    let y0 = y.floor() as usize;
    let x1 = (x0 + 1).min(gw - 1);
    let y1 = (y0 + 1).min(gh - 1);
    let fx = x - x0 as f32;
    let fy = y - y0 as f32;

    let c00 = colors[y0 * gw + x0];
    let c10 = colors[y0 * gw + x1];
    let c01 = colors[y1 * gw + x0];
    let c11 = colors[y1 * gw + x1];

    let r = c00[0] * (1.0 - fx) * (1.0 - fy)
          + c10[0] * fx * (1.0 - fy)
          + c01[0] * (1.0 - fx) * fy
          + c11[0] * fx * fy;
    let g = c00[1] * (1.0 - fx) * (1.0 - fy)
          + c10[1] * fx * (1.0 - fy)
          + c01[1] * (1.0 - fx) * fy
          + c11[1] * fx * fy;
    let b = c00[2] * (1.0 - fx) * (1.0 - fy)
          + c10[2] * fx * (1.0 - fy)
          + c01[2] * (1.0 - fx) * fy
          + c11[2] * fx * fy;
    (r, g, b)
}