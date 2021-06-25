extern crate nalgebra_glm as glm;
extern crate tinyfiledialogs as tfd;
extern crate ozy_engine as ozy;

use std::path::Path;
use std::mem::size_of;
use std::process::{exit};
use glfw::{Action, Context, MouseButton, WindowEvent, WindowMode};
use imgui::{Condition, DrawCmd, FontAtlasRefMut, TextureId, im_str};
use ozy::glutil;
use ozy::render::clip_from_screen;
use gl::types::*;

const DEFAULT_TEX_PARAMS: [(GLenum, GLenum); 4] = [
    (gl::TEXTURE_WRAP_S, gl::REPEAT),
    (gl::TEXTURE_WRAP_T, gl::REPEAT),
    (gl::TEXTURE_MIN_FILTER, gl::LINEAR),
    (gl::TEXTURE_MAG_FILTER, gl::LINEAR)
];

struct OpenImage {
    name: String,
    gl_name: GLuint,
    width: usize,
    height: usize
}

/*
Probably the prepared statements to use:

SELECT name FROM tags;

SELECT path FROM images
JOIN
  (SELECT image_id FROM image_tags
  WHERE image_tags.tag_id = (
        SELECT id FROM tags WHERE name="persona"
      ))
WHERE id=image_id;
*/

fn main() {
    let mut window_size = glm::vec2(1080, 1080);

    //Init glfw and create the window
    let mut glfw = match glfw::init(glfw::FAIL_ON_ERRORS) {
        Ok(g) => { g }
        Err(e) => {
            println!("Error initializing GLFW: {}", e);
            return;
        }
    };
    let (mut window, events) = glfw.create_window(window_size.x, window_size.y, "uwu_db", WindowMode::Windowed).unwrap();
    window.set_key_polling(true);
    window.set_mouse_button_polling(true);
    window.set_cursor_pos_polling(true);
    window.set_scroll_polling(true);
    window.set_framebuffer_size_polling(true);

    //Load OpenGL functions
    gl::load_with(|symbol| window.get_proc_address(symbol));

    //OpenGL static config
    unsafe {        
		gl::Enable(gl::FRAMEBUFFER_SRGB); 								//Enable automatic linear->SRGB space conversion
        gl::Enable(gl::BLEND);											//Enable alpha blending
		gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);			//Set blend func to (Cs * alpha + Cd * (1.0 - alpha))
        
        #[cfg(gloutput)]
		{
            use std::ptr;
			gl::Enable(gl::DEBUG_OUTPUT);									                                    //Enable verbose debug output
			gl::Enable(gl::DEBUG_OUTPUT_SYNCHRONOUS);						                                    //Synchronously call the debug callback function
			gl::DebugMessageCallback(Some(ozy::glutil::gl_debug_callback), ptr::null());		                        //Register the debug callback
			gl::DebugMessageControl(gl::DONT_CARE, gl::DONT_CARE, gl::DONT_CARE, 0, ptr::null(), gl::TRUE);
		}
    }

    //Compile IMGUI shader
    let imgui_program = match glutil::compile_program_from_files("shaders/imgui.vert", "shaders/imgui.frag") {
        Ok(shader) => { shader }
        Err(e) => {
            println!("Error compiling shader: {}", e);
            exit(-1);
        }
    };
    
    //Creating Dear ImGui context
    let mut imgui_context = imgui::Context::create();
    imgui_context.style_mut().use_dark_colors();
    {
        let io = imgui_context.io_mut();
        io.display_size[0] = window.get_size().0 as f32;
        io.display_size[1] = window.get_size().1 as f32;
    }

    //Create and upload Dear IMGUI font atlas
    match imgui_context.fonts() {
        FontAtlasRefMut::Owned(atlas) => unsafe {
            let mut tex = 0;
            let font_atlas = atlas.build_rgba32_texture();

            let tex_params = [
                (gl::TEXTURE_WRAP_S, gl::REPEAT),
                (gl::TEXTURE_WRAP_T, gl::REPEAT),
                (gl::TEXTURE_MIN_FILTER, gl::LINEAR),
                (gl::TEXTURE_MAG_FILTER, gl::LINEAR)
            ];

            gl::GenTextures(1, &mut tex);
            gl::BindTexture(gl::TEXTURE_2D, tex);            
            glutil::apply_texture_parameters(&tex_params);
            gl::TexImage2D(gl::TEXTURE_2D, 0, gl::RGBA as GLsizei, font_atlas.width as GLsizei, font_atlas.height as GLsizei, 0, gl::RGBA, gl::UNSIGNED_BYTE, font_atlas.data.as_ptr() as _);
            atlas.tex_id = TextureId::new(tex as usize);
        }
        FontAtlasRefMut::Shared(_) => {
            panic!("Not dealing with this case.");
        }
    };

    //Open a connection to the database
    let db_name = "uwu.db";
    let connection = if !Path::new(db_name).exists() {
        let con = sqlite::open(db_name).unwrap();
        
        con.execute(
            "
            CREATE TABLE images (id INTEGER, path STRING NOT NULL UNIQUE, PRIMARY KEY (id));
            CREATE TABLE tags (id INTEGER, name STRING NOT NULL UNIQUE, PRIMARY KEY (id));
            CREATE TABLE image_tags (image_id INTEGER, tag_id INTEGER);
            "
        ).unwrap();
        
        con
    } else {
        sqlite::open(db_name).unwrap()
    };

    let mut open_images: Vec<OpenImage> = vec![];

    while !window.should_close() {
        let imgui_io = imgui_context.io_mut();

        glfw.poll_events();
        for (_, event) in glfw::flush_messages(&events) {
            match event {
                WindowEvent::Close => { window.set_should_close(true); }
                WindowEvent::FramebufferSize (x, y) => {
                    window_size.x = x as u32;
                    window_size.y = y as u32;
                    imgui_io.display_size[0] = x as f32;
                    imgui_io.display_size[1] = y as f32;
                }
                WindowEvent::MouseButton (button, action, ..) => {
                    let idx = match button {
                        MouseButton::Button1 => { 0 }
                        MouseButton::Button2 => { 1 }
                        MouseButton::Button3 => { 2 }
                        MouseButton::Button4 => { 3 }
                        MouseButton::Button5 => { 4 }
                        MouseButton::Button6 => { 5 }
                        MouseButton::Button7 => { 6 }
                        MouseButton::Button8 => { 7 }
                    };

                    match action {
                        Action::Press => {
                            imgui_io.mouse_down[idx] = true;
                        }
                        Action::Release => {
                            imgui_io.mouse_down[idx] = false;
                        }
                        _ => {}
                    }
                }
                WindowEvent::CursorPos(x, y) => {
                    imgui_io.mouse_pos[0] = x as f32;
                    imgui_io.mouse_pos[1] = y as f32;
                }
                WindowEvent::Scroll(x, y) => {
                    imgui_io.mouse_wheel = y as f32;
                    imgui_io.mouse_wheel_h = x as f32;
                }
                _ => { println!("Unhandled event: {:?}", event); }
            }
        }

        let imgui_ui = imgui_context.frame();
        if let Some(token) = imgui::Window::new(im_str!("uwu_db"))
                            .position([0.0, 0.0], Condition::Always)
                            .size([window_size.x as f32, window_size.y as f32], Condition::Always)
                            .resizable(false)
                            .collapsible(false)
                            .title_bar(false)
                            .begin(&imgui_ui) {

            for i in 0..open_images.len() {
                let im = &open_images[i];

                let max_width = window_size.x / 2;
                let factor = if im.width > max_width as usize {
                    max_width as f32 / im.width as f32
                } else {
                    1.0
                };
                imgui::Image::new(imgui::TextureId::new(im.gl_name as usize), [im.width as f32 * factor, im.height as f32 * factor]).build(&imgui_ui);
                imgui_ui.same_line(0.0);
                imgui_ui.text(&im.name);

                if imgui_ui.button(im_str!("Set tags"), [0.0, 32.0]) {
                }
                imgui_ui.same_line(0.0);

                if imgui_ui.button(im_str!("Remove"), [0.0, 32.0]) {

                }
            }
            imgui_ui.separator();
                            
            if imgui_ui.button(im_str!("Open image"), [0.0, 32.0]) {
                if let Some(image_paths) = tfd::open_file_dialog_multi("Open image", "L:\\images\\", Some((&["*.png", "*.jpg"], ".png, .jpg"))) {
                    for path in image_paths {
                        println!("Trying to load: {}", path);
                        let image_data = glutil::image_data_from_path(&path, glutil::ColorSpace::Gamma);
                        let height = image_data.height;
                        let width = image_data.width;
                        let image_gl_name = unsafe { glutil::load_texture_from_data(image_data, &DEFAULT_TEX_PARAMS) };
                        println!("Loaded successfully.");

                        let o = OpenImage {
                            name: path,
                            gl_name: image_gl_name,
                            width: width as usize,
                            height: height as usize
                        };
                        open_images.push(o);
                    }
                }
            }
            imgui_ui.same_line(0.0);

            if imgui_ui.button(&im_str!("Exit"), [0.0, 32.0]) {
                window.set_should_close(true);
            }

            token.end(&imgui_ui);
        }

        //Rendering Dear IMGUI
        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT);
            gl::Viewport(0, 0, window_size.x as GLint, window_size.y as GLint);
            gl::UseProgram(imgui_program);
            glutil::bind_matrix4(imgui_program, "projection", &clip_from_screen(window_size));
            {
                let draw_data = imgui_ui.render();
                if draw_data.total_vtx_count > 0 {
                    for list in draw_data.draw_lists() {
                        let vert_size = 8;
                        let mut verts = vec![0.0; list.vtx_buffer().len() * vert_size];

                        let mut current_vertex = 0;
                        let vtx_buffer = list.vtx_buffer();
                        for vtx in vtx_buffer.iter() {
                            let idx = current_vertex * vert_size;
                            verts[idx] = vtx.pos[0];
                            verts[idx + 1] = vtx.pos[1];
                            verts[idx + 2] = vtx.uv[0];
                            verts[idx + 3] = vtx.uv[1];    
                            verts[idx + 4] = vtx.col[0] as f32 / 255.0;
                            verts[idx + 5] = vtx.col[1] as f32 / 255.0;
                            verts[idx + 6] = vtx.col[2] as f32 / 255.0;
                            verts[idx + 7] = vtx.col[3] as f32 / 255.0;
    
                            current_vertex += 1;
                        }

                        let imgui_vao = glutil::create_vertex_array_object(&verts, list.idx_buffer(), &[2, 2, 4]);

                        for command in list.commands() {
                            match command {
                                DrawCmd::Elements {count, cmd_params} => {
                                    gl::BindVertexArray(imgui_vao);
                                    gl::ActiveTexture(gl::TEXTURE0);
                                    gl::BindTexture(gl::TEXTURE_2D, cmd_params.texture_id.id() as GLuint);
                                    gl::Scissor(cmd_params.clip_rect[0] as GLint,
                                                window_size[1] as GLint - cmd_params.clip_rect[3] as GLint,
                                                (cmd_params.clip_rect[2] - cmd_params.clip_rect[0]) as GLint,
                                                (cmd_params.clip_rect[3] - cmd_params.clip_rect[1]) as GLint
                                    );
                                    gl::DrawElementsBaseVertex(gl::TRIANGLES, count as GLint, gl::UNSIGNED_SHORT, (cmd_params.idx_offset * size_of::<GLushort>()) as _, cmd_params.vtx_offset as GLint);
                                }
                                DrawCmd::ResetRenderState => { println!("DrawCmd::ResetRenderState."); }
                                DrawCmd::RawCallback {..} => { println!("DrawCmd::RawCallback."); }
                            }
                        }
                        
                        //Free the vertex and index buffers
                        let mut bufs = [0, 0];
                        gl::GetIntegerv(gl::ARRAY_BUFFER_BINDING, &mut bufs[0]);
                        gl::GetIntegerv(gl::ELEMENT_ARRAY_BUFFER_BINDING, &mut bufs[1]);
                        let bufs = [bufs[0] as GLuint, bufs[1] as GLuint];
                        gl::DeleteBuffers(2, &bufs[0]);
                        gl::DeleteVertexArrays(1, &imgui_vao);
                    }
                }
            }
        }

        window.swap_buffers();
    }
}
