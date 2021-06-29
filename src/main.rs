extern crate nalgebra_glm as glm;
extern crate tinyfiledialogs as tfd;
extern crate ozy_engine as ozy;

use sqlite::State;
use std::path::Path;
use std::mem::size_of;
use std::process::{exit};
use std::thread;
use std::sync::mpsc;
use glfw::{Action, Context, Key, MouseButton, WindowEvent, WindowHint, WindowMode};
use imgui::{Condition, DrawCmd, FontAtlasRefMut, ImStr, MenuItem, TextureId, im_str};
use ozy::glutil;
use ozy::render::{clip_from_screen};
use ozy::structs::ImageData;
use gl::types::*;
use tfd::MessageBoxIcon;

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

impl OpenImage {
    fn from_path(path: String) -> Self {
        println!("Trying to load: {}", path);
        let image_data = glutil::image_data_from_path(&path, glutil::ColorSpace::Gamma);
        let height = image_data.height;
        let width = image_data.width;
        let gl_name = unsafe { glutil::load_texture_from_data(image_data, &DEFAULT_TEX_PARAMS) };
        println!("Loaded successfully.");
        OpenImage {
            name: path,
            gl_name,
            width: width as usize,
            height: height as usize
        }
    }

    fn from_imagedata(image_data: ImageData, path: String) -> Self {
        let height = image_data.height;
        let width = image_data.width;
        let gl_name = unsafe { glutil::load_texture_from_data(image_data, &DEFAULT_TEX_PARAMS) };
        OpenImage {
            name: path,
            gl_name,
            width: width as usize,
            height: height as usize 
        }
    }
}

fn load_openimage(open_images: &mut Vec<OpenImage>, path: String) {
    open_images.push(OpenImage::from_path(path));
}

/*
Prepared statements for later:

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
    glfw.window_hint(WindowHint::RefreshRate(Some(60)));
    let (mut window, events) = glfw.create_window(window_size.x, window_size.y, "uwu_db", WindowMode::Windowed).unwrap();
    window.set_key_polling(true);
    window.set_mouse_button_polling(true);
    window.set_cursor_pos_polling(true);
    window.set_scroll_polling(true);
    window.set_framebuffer_size_polling(true);
    window.set_drag_and_drop_polling(true);
    window.set_char_polling(true);

    //Load OpenGL functions
    gl::load_with(|symbol| window.get_proc_address(symbol));

    //OpenGL static config
    unsafe {        
		gl::Enable(gl::FRAMEBUFFER_SRGB); 								//Enable automatic linear->SRGB space conversion
        gl::Enable(gl::BLEND);											//Enable alpha blending
        gl::Enable(gl::SCISSOR_TEST);
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
            tfd::message_box_ok("GLSL compilation error", &format!("Unable to compile the GL shader:\n{}", e), MessageBoxIcon::Error);
            exit(-1);
        }
    };
    
    //Creating Dear ImGui context
    let mut imgui_context = imgui::Context::create();

    //Imgui IO init
    {
        let io = imgui_context.io_mut();
        io.display_size[0] = window.get_size().0 as f32;
        io.display_size[1] = window.get_size().1 as f32;

        //Set up keyboard map
        io.key_map[imgui::Key::Tab as usize] = Key::Tab as u32;
        io.key_map[imgui::Key::LeftArrow as usize] = Key::Left as u32;
        io.key_map[imgui::Key::RightArrow as usize] = Key::Right as u32;
        io.key_map[imgui::Key::UpArrow as usize] = Key::Up as u32;
        io.key_map[imgui::Key::DownArrow as usize] = Key::Down as u32;
        io.key_map[imgui::Key::PageDown as usize] = Key::PageDown as u32;
        io.key_map[imgui::Key::PageUp as usize] = Key::PageUp as u32;
        io.key_map[imgui::Key::Home as usize] = Key::Home as u32;
        io.key_map[imgui::Key::End as usize] = Key::End as u32;
        io.key_map[imgui::Key::Insert as usize] = Key::Insert as u32;
        io.key_map[imgui::Key::Delete as usize] = Key::Delete as u32;
        io.key_map[imgui::Key::Backspace as usize] = Key::Backspace as u32;
        io.key_map[imgui::Key::Space as usize] = Key::Space as u32;
        io.key_map[imgui::Key::Enter as usize] = Key::Enter as u32;
        io.key_map[imgui::Key::KeyPadEnter as usize] = Key::KpEnter as u32;
        io.key_map[imgui::Key::A as usize] = Key::A as u32;
        io.key_map[imgui::Key::C as usize] = Key::C as u32;
        io.key_map[imgui::Key::V as usize] = Key::V as u32;
        io.key_map[imgui::Key::X as usize] = Key::X as u32;
        io.key_map[imgui::Key::Y as usize] = Key::Y as u32;
        io.key_map[imgui::Key::Z as usize] = Key::Z as u32;
    }

    //Imgui style init
    {
        let style = imgui_context.style_mut();
        style.use_dark_colors();
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
        
        //Initialize the tables
        con.execute(
            "
            CREATE TABLE images (id INTEGER, path STRING NOT NULL UNIQUE, PRIMARY KEY (id));
            CREATE TABLE tags (id INTEGER, name STRING NOT NULL UNIQUE, PRIMARY KEY (id));
            CREATE TABLE image_tags (image_id INTEGER, tag_id INTEGER);

            INSERT INTO tags (name) VALUES (\"persona\");
            "
        ).unwrap();
        
        con
    } else {
        sqlite::open(db_name).unwrap()
    };

    let tags = {
        let mut tag_statement = connection.prepare(
            "
            SELECT name FROM tags;
            "
        ).unwrap();

        let mut ts = Vec::new();
        while let State::Row = tag_statement.next().unwrap() {
            let the_string = tag_statement.read::<String>(0).unwrap();            
            ts.push(the_string);
        }
        ts
    };
    println!("{:?}", tags);

    let mut open_images: Vec<OpenImage> = vec![];
    let mut text_buffer = imgui::ImString::with_capacity(256);    
    //let mut tags = Vec::new();
    let mut selected_tag = 0;
    let mut selected_image = None;

    let (path_tx, path_rx): (mpsc::Sender<String>, mpsc::Receiver<String>) = mpsc::channel();
    let (openimage_tx, openimage_rx) = mpsc::channel();
    thread::spawn(move || {
        loop {
            while let Ok(path) = path_rx.try_recv() {
                let image_data = glutil::image_data_from_path(&path, glutil::ColorSpace::Gamma);
                openimage_tx.send((image_data, path)).unwrap();
            }
        }
    });

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
                WindowEvent::Key(key, _, action, ..) => {
                    if action == Action::Press {
                        imgui_io.keys_down[key as usize] = true;
                    } else if action == Action::Release {
                        imgui_io.keys_down[key as usize] = false;
                    }
                }
                WindowEvent::Char(c) => {
                    imgui_io.add_input_character(c);
                }
                WindowEvent::FileDrop(file_paths) => {
                    for path in file_paths {
                        let s = String::from(path.to_str().unwrap());
                        path_tx.send(s).unwrap();
                    }
                }
                _ => { println!("Unhandled event: {:?}", event); }
            }
        }

        //Set mod keys for this frame
        imgui_io.key_ctrl = imgui_io.keys_down[Key::LeftControl as usize] || imgui_io.keys_down[Key::RightControl as usize];
        imgui_io.key_shift = imgui_io.keys_down[Key::LeftShift as usize] || imgui_io.keys_down[Key::RightShift as usize];
        imgui_io.key_alt = imgui_io.keys_down[Key::LeftAlt as usize] || imgui_io.keys_down[Key::RightAlt as usize];        

        //Begin Imgui drawing
        let imgui_ui = imgui_context.frame();

        //Receive images from the image loading thread
        while let Ok((image, path)) = openimage_rx.try_recv() {
            let o = OpenImage::from_imagedata(image, path);
            open_images.push(o);
        }

        let menu_height = 50.0;
        if let Some(token) = imgui::Window::new(im_str!("menu"))
                             .position([0.0, 0.0], Condition::Always)
                             .size([window_size.x as f32, menu_height], Condition::Always)
                             .resizable(false)
                             .collapsible(false)
                             .title_bar(false)
                             .begin(&imgui_ui) {
            
            imgui::ComboBox::new(im_str!("Active tag")).build_simple_string(&imgui_ui, &mut selected_tag, &[im_str!("A"), im_str!("fucking"), im_str!("list")]);
            imgui_ui.same_line(0.0);
                                
            if imgui_ui.button(im_str!("Open image(s)"), [0.0, 32.0]) {
                if let Some(image_paths) = tfd::open_file_dialog_multi("Open image", "L:\\images\\", Some((&["*.png", "*.jpg"], ".png, .jpg"))) {
                    for path in image_paths {
                        if let Err(e) = path_tx.send(path) {
                            println!("{}", e);
                        }
                        //load_openimage(&mut open_images, path);
                    }
                }
            }
            imgui_ui.same_line(0.0);
            
            if imgui_ui.button(&im_str!("Exit"), [0.0, 32.0]) {
                window.set_should_close(true);
            }

            token.end(&imgui_ui);
        }

        //Draw main window where images are displayed
        if let Some(token) = imgui::Window::new(im_str!("uwu_db"))
                            .position([0.0, menu_height], Condition::Always)
                            .size([window_size.x as f32, window_size.y as f32 - menu_height], Condition::Always)
                            .resizable(false)
                            .collapsible(false)
                            .title_bar(false)
                            .horizontal_scrollbar(true)
                            //.menu_bar(true)
                            .begin(&imgui_ui) {

            if false {
                imgui_ui.set_scroll_y(imgui_ui.scroll_y() + 1.0);
                if imgui_ui.scroll_y() >= imgui_ui.scroll_max_y() {
                    imgui_ui.set_scroll_y(0.0);
                }
            }

            let pics_per_row = 4;
            for i in 0..open_images.len() {
                let im = &open_images[i];
                let max_width = window_size.x as f32 / pics_per_row as f32 - 24.0;
                let factor = if im.width > max_width as usize {
                    max_width as f32 / im.width as f32
                } else {
                    1.0
                };
                if imgui::ImageButton::new(imgui::TextureId::new(im.gl_name as usize), [im.width as f32 * factor, im.height as f32 * factor]).build(&imgui_ui) {
                    println!("Clicked on {}", im.name);
                    selected_image = Some(i);
                }
                imgui_ui.same_line(0.0);
                if i % pics_per_row == pics_per_row - 1 {
                    imgui_ui.new_line();
                }
            }

            token.end(&imgui_ui);
        }

        if let Some(image) = selected_image {
            let im = &open_images[image];
            if let Some(token) = imgui::Window::new(im_str!("Image control panel")).begin(&imgui_ui) {
                if !imgui_ui.is_window_focused() {
                    selected_image = None;
                }

                imgui::InputText::new(&imgui_ui, im_str!("New tag"), &mut text_buffer).build();
                imgui_ui.button(im_str!("Submit"), [0.0, 32.0]);

                token.end(&imgui_ui);
            }
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
