extern crate nalgebra_glm as glm;
extern crate tinyfiledialogs as tfd;
extern crate ozy_engine as ozy;

use sqlite::State;
use core::ops::RangeInclusive;
use std::path::Path;
use std::mem::size_of;
use std::process::{exit};
use std::thread;
use std::time::{Duration, Instant};
use std::sync::mpsc;
use glfw::{Action, Context, Key, MouseButton, WindowEvent, WindowHint, WindowMode};
use imgui::{Condition, DrawCmd, FontAtlasRefMut, ImStr, ImString, MenuItem, TextureId, WindowFocusedFlags, im_str};
use ozy::glutil;
use ozy::render::{clip_from_screen};
use ozy::structs::ImageData;
use gl::types::*;
use tfd::MessageBoxIcon;

use crate::structs::OpenImage;

mod structs;

const DEFAULT_TEX_PARAMS: [(GLenum, GLenum); 4] = [
    (gl::TEXTURE_WRAP_S, gl::REPEAT),
    (gl::TEXTURE_WRAP_T, gl::REPEAT),
    (gl::TEXTURE_MIN_FILTER, gl::LINEAR),
    (gl::TEXTURE_MAG_FILTER, gl::LINEAR)
];

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

fn imstr_ref_array(strs: &Vec<ImString>) -> Vec<&ImString> {    
    let mut tag_refs = Vec::with_capacity(strs.len());
    for t in strs {
        tag_refs.push(t);
    }
    tag_refs
}

fn insert_tag(strs: &mut Vec<ImString>, new_str: &ImString) {                    
    if !strs.contains(&new_str) {
        strs.push(new_str.clone());
        strs.sort();
    }
}

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
            let font_atlas = atlas.build_rgba32_texture();

            let tex_params = [
                (gl::TEXTURE_WRAP_S, gl::REPEAT),
                (gl::TEXTURE_WRAP_T, gl::REPEAT),
                (gl::TEXTURE_MIN_FILTER, gl::NEAREST),
                (gl::TEXTURE_MAG_FILTER, gl::NEAREST)
            ];

            let mut tex = 0;
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
            
            INSERT INTO tags (name) VALUES (\"Persona\");
            "
        ).unwrap();
        
        con
    } else { sqlite::open(db_name).unwrap() };

    //Fetch tags from database
    let mut tags = {
        let mut tag_statement = connection.prepare(
            "
            SELECT name FROM tags ORDER BY name;
            "
        ).unwrap();

        let mut ts = Vec::new();
        while let State::Row = tag_statement.next().unwrap() {
            let the_string = tag_statement.read::<String>(0).unwrap();
            ts.push(im_str!("{}", the_string));
        }
        ts
    };

    let dropdown_width = 150.0;
    let mut open_images: Vec<OpenImage> = vec![];
    let mut new_tag_buffer = imgui::ImString::with_capacity(256);
    let mut selected_tag = 0;
    let mut control_panel_tag = 0;
    let mut selected_image = None;
    let mut pics_per_row: u32 = 3;
    let mut auto_scroll = false;

    let (path_tx, path_rx): (mpsc::Sender<String>, mpsc::Receiver<String>) = mpsc::channel();
    let (openimage_tx, openimage_rx) = mpsc::channel();
    thread::spawn(move || {
        while let Ok(path) = path_rx.recv() {
            let image_data = glutil::image_data_from_path(&path, glutil::ColorSpace::Gamma);
            openimage_tx.send((image_data, path)).unwrap();
        }
    });

    let mut frame_timer = ozy::structs::FrameTimer::new();
    while !window.should_close() {
        let delta_time = {
            let frame_instant = Instant::now();
            let dur = frame_instant.duration_since(frame_timer.last_frame_instant);
            frame_timer.last_frame_instant = frame_instant;
            dur.as_secs_f32()
        };        

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
                        if let Err(e) = path_tx.send(s) {
                            println!("{}", e);
                        }
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

        //Top menu
        let menu_height = 50.0;
        if let Some(token) = imgui::Window::new(im_str!("menu"))
                             .position([0.0, 0.0], Condition::Always)
                             .size([window_size.x as f32, menu_height], Condition::Always)
                             .resizable(false)
                             .collapsible(false)
                             .title_bar(false)
                             .begin(&imgui_ui) {

            fn clear_open_images(images: &mut Vec<OpenImage>, selected_image: &mut Option<usize>) {
                *selected_image = None;
                images.clear();                
            }
            
            imgui_ui.set_next_item_width(dropdown_width);
            if imgui::ComboBox::new(im_str!("Active tag")).build_simple_string(&imgui_ui, &mut selected_tag, imstr_ref_array(&tags).as_slice()) {
                let tag = &tags[selected_tag];
                clear_open_images(&mut open_images, &mut selected_image);

                //Bind SQL variables
            }
            imgui_ui.same_line(0.0);

            imgui_ui.set_next_item_width(200.0);
            imgui::Slider::new(im_str!("Images per row")).range(RangeInclusive::new(1, 16)).build(&imgui_ui, &mut pics_per_row);
            imgui_ui.same_line(0.0);

            if imgui_ui.button(im_str!("Open image(s)"), [0.0, 32.0]) {
                if let Some(image_paths) = tfd::open_file_dialog_multi("Open image", "L:/images/", Some((&["*.png", "*.jpg"], ".png, .jpg"))) {
                    for path in image_paths {
                        if let Err(e) = path_tx.send(path) {
                            println!("{}", e);
                        }
                    }
                }
            }
            imgui_ui.same_line(0.0);
                                
            if imgui_ui.button(im_str!("Clear images"), [0.0, 32.0]) {
                clear_open_images(&mut open_images, &mut selected_image);
            }
            imgui_ui.same_line(0.0);
                                
            if imgui_ui.button(im_str!("Toggle scrolling"), [0.0, 32.0]) {
                auto_scroll = !auto_scroll;
            }
            imgui_ui.same_line(0.0);
            
            if imgui_ui.button(&im_str!("Exit"), [0.0, 32.0]) {
                window.set_should_close(true);
            }

            token.end(&imgui_ui);
        }

        //Draw main window where images are displayed
        let mut focus_control_panel = false;    //Gets set if the user selected an image this frame
        if let Some(token) = imgui::Window::new(im_str!("uwu_db"))
                            .position([0.0, menu_height], Condition::Always)
                            .size([window_size.x as f32, window_size.y as f32 - menu_height], Condition::Always)
                            .resizable(false)
                            .collapsible(false)
                            .title_bar(false)
                            .horizontal_scrollbar(true)
                            //.menu_bar(true)
                            .begin(&imgui_ui) {

            if auto_scroll {
                let dist = 200.0 * delta_time;
                imgui_ui.set_scroll_y(imgui_ui.scroll_y() + dist);
                if imgui_ui.scroll_y() >= imgui_ui.scroll_max_y() {
                    imgui_ui.set_scroll_y(0.0);
                }
            }

            for i in 0..open_images.len() {
                let im = &open_images[i];
                let max_width = window_size.x as f32 / pics_per_row as f32 - 24.0;
                /*
                let factor = if im.width > max_width as usize {
                    max_width as f32 / im.width as f32
                } else {
                    1.0
                };
                */
                let factor = max_width as f32 / im.width as f32;
                if imgui::ImageButton::new(imgui::TextureId::new(im.gl_name as usize), [im.width as f32 * factor, im.height as f32 * factor]).build(&imgui_ui) {
                    selected_image = Some(i);
                    focus_control_panel = true;
                }
                imgui_ui.same_line(0.0);
                if i as u32 % pics_per_row == pics_per_row - 1 {
                    imgui_ui.new_line();
                }
            }

            token.end(&imgui_ui);
        }

        //Image control panel window
        if let Some(image) = selected_image {
            fn close_window(selected_image: &mut Option<usize>) {
                *selected_image = None;
            }

            let im = &mut open_images[image];
            let mut to_remove = None;
            if let Some(token) = imgui::Window::new(im_str!("Image control panel"))
                                 .collapsible(false)
                                 .position([(window_size.x / 2) as f32, (window_size.y / 2) as f32], Condition::Once)
                                 .focused(focus_control_panel)
                                 .begin(&imgui_ui) {
                if !imgui_ui.is_window_focused_with_flags(WindowFocusedFlags::CHILD_WINDOWS) {
                    close_window(&mut selected_image);
                }

                imgui_ui.text(im_str!("{}", im.name));

                if imgui_ui.button(im_str!("Remove this image"), [0.0, 32.0]) {
                    to_remove = Some(image);
                    close_window(&mut selected_image);
                }
                imgui_ui.separator();

                imgui::InputText::new(&imgui_ui, im_str!("New tag"), &mut new_tag_buffer).build();
                if imgui_ui.button(im_str!("Create tag and apply to image"), [0.0, 32.0]) {
                    let new_tag = new_tag_buffer.clone();
                    new_tag_buffer.clear();
                    
                    //Do SQL
                    if !tags.contains(&new_tag) {
                        connection.execute(format!(
                            "
                            INSERT OR IGNORE INTO tags (name) VALUES (\"{}\");
                            INSERT OR IGNORE INTO images (path) VALUES (\"{}\");
                            INSERT OR IGNORE INTO image_tags VALUES (
                                    (SELECT id FROM images WHERE path=\"{}\")
                                ,   (SELECT id FROM tags WHERE name=\"{}\")
                                );
                            ", new_tag.to_str(), im.name, im.name, new_tag.to_str())
                        ).unwrap();
                    }

                    //Insert tags into appropriate arrays
                    insert_tag(&mut tags, &new_tag);
                    insert_tag(&mut im.tags, &new_tag);
                }
                imgui_ui.separator();

                imgui_ui.set_next_item_width(dropdown_width);
                imgui::ComboBox::new(im_str!("Extant tags")).build_simple_string(&imgui_ui, &mut control_panel_tag, imstr_ref_array(&tags).as_slice());
                if imgui_ui.button(im_str!("Apply tag to image"), [0.0, 32.0]) {
                    let r = tags[control_panel_tag].clone();
                    insert_tag(&mut im.tags, &r);
                }
                imgui_ui.separator();

                imgui_ui.text(im_str!("Click on a tag to remove it from this image:"));
                let mut to_remove = None;
                for i in 0..im.tags.len() {
                    if imgui_ui.checkbox(&im.tags[i], &mut true) {
                        to_remove = Some(i);
                    }
                }
                if let Some(idx) = to_remove {
                    im.tags.remove(idx);
                }

                token.end(&imgui_ui);
            }
            if let Some(index) = to_remove {            
                open_images.remove(index);
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
