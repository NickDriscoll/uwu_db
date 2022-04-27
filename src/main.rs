 //Alias the long library names
extern crate nalgebra_glm as glm;
extern crate tinyfiledialogs as tfd;
extern crate ozy_engine as ozy;

use sqlite::State;
use std::path::Path;
use std::mem::size_of;
use std::process::{exit};
use std::{fs, thread};
use std::sync::mpsc;
use glfw::{Action, Context, Key, MouseButton, WindowEvent, WindowMode};
use imgui::{Condition, DrawCmd, FontAtlasRefMut, ImageButton, ImString, MenuItem, TextureId, WindowFocusedFlags};
use ozy::glutil;
use ozy::render::{clip_from_screen};
use gl::types::*;
use tfd::{MessageBoxIcon, YesNo};

use crate::structs::*;

mod structs;

//Texture parameters that the images will all use
const DEFAULT_TEX_PARAMS: [(GLenum, GLenum); 4] = [
    (gl::TEXTURE_WRAP_S, gl::REPEAT),
    (gl::TEXTURE_WRAP_T, gl::REPEAT),
    (gl::TEXTURE_MIN_FILTER, gl::LINEAR),
    (gl::TEXTURE_MAG_FILTER, gl::LINEAR)
];

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

fn send_or_error<T>(tx: &mpsc::Sender<T>, item: T) {
    if let Err(e) = tx.send(item) {
        println!("{}", e);
    }
}

fn recompute_selected_tags(selected_image_tags: &mut Vec<bool>, tags: &Vec<ImString>, image_tags: &Vec<ImString>) {    
    for i in 0..tags.len() {
        selected_image_tags[i] = image_tags.contains(&tags[i]);
    }
}

fn clear_open_images(images: &mut Vec<OpenImage>, selected_image: &mut Option<usize>) {
    *selected_image = None;
    images.clear();                
}

fn main() {
    let mut window_size = glm::vec2(1280, 720);
    let mut image_directory = String::from("E:/images/good");

    //Init glfw and create the window
    let mut glfw = match glfw::init(glfw::FAIL_ON_ERRORS) {
        Ok(g) => { g }
        Err(e) => {
            tfd::message_box_ok("Error initializing GLFW", &format!("Error initializing GLFW: {}", e), MessageBoxIcon::Error);            
            return;
        }
    };
    //glfw.window_hint(WindowHint::RefreshRate(Some(60)));
    let (mut window, events) = glfw.create_window(window_size.x, window_size.y, "uwu_db", WindowMode::Windowed).unwrap();

    //Enable polling for the events we wish to receive
    window.set_key_polling(true);
    window.set_mouse_button_polling(true);
    window.set_cursor_pos_polling(true);
    window.set_scroll_polling(true);
    window.set_framebuffer_size_polling(true);
    window.set_drag_and_drop_polling(true);
    window.set_char_polling(true);

    //Initialize OpenGL function pointers
    //gl::load_with() takes a closure that returns an OpenGL procedure's address given its name
    //GLFW's Window::get_proc_address() does just that, so we can effortless plug them into each other with this line
    gl::load_with(|symbol| window.get_proc_address(symbol));

    //Compile IMGUI shader
    let imgui_program = match glutil::compile_program_from_files(&[(gl::VERTEX_SHADER, "shaders/imgui.vert"), (gl::FRAGMENT_SHADER, "shaders/imgui.frag")]) {
        Ok(shader) => { shader }
        Err(e) => {
            tfd::message_box_ok("GLSL compilation error", &format!("Unable to compile the GL shader:\n{}", e), MessageBoxIcon::Error);
            exit(-1);
        }
    };

    //OpenGL static config
    unsafe {
		gl::Enable(gl::FRAMEBUFFER_SRGB); 								        //Enable automatic linear->SRGB space conversion
        gl::Enable(gl::SCISSOR_TEST);                                           //Enable scissor test for GUI clipping
        gl::Enable(gl::BLEND);											        //Enable alpha blending
		gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);			        //Set blend func to (Cs * alpha + Cd * (1.0 - alpha))
        gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1);                               //Sets the texture alignment requirements to be byte-aligned
        gl::UseProgram(imgui_program);                                          //This program only uses one GL program, so we can bind it once in advance
        
        //These options are only compiled in if the 'gloutput' flag is passed
        #[cfg(gloutput)]
		{
            use std::ptr;
			gl::Enable(gl::DEBUG_OUTPUT);									                                    //Enable verbose debug output
			gl::Enable(gl::DEBUG_OUTPUT_SYNCHRONOUS);						                                    //Synchronously call the debug callback function
			gl::DebugMessageCallback(Some(ozy::glutil::gl_debug_callback), ptr::null());		                        //Register the debug callback
			gl::DebugMessageControl(gl::DONT_CARE, gl::DONT_CARE, gl::DONT_CARE, 0, ptr::null(), gl::TRUE);
		}
    }
    
    let mut imgui_context = imgui::Context::create();       //Creating Dear ImGui context
    imgui_context.style_mut().use_dark_colors();            //Just using Dear Imgui's default dark style for now

    //Imgui IO init
    {
        let io = imgui_context.io_mut();
        io.display_size[0] = window.get_size().0 as f32;
        io.display_size[1] = window.get_size().1 as f32;

        //Set up keyboard index map
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

    //Create and upload Dear IMGUI font atlas
    match imgui_context.fonts() {
        FontAtlasRefMut::Owned(atlas) => unsafe {
            let font_atlas = atlas.build_rgba32_texture();      //We need an RGBA texture in order for the images themselves to be rendered with color

            //We sample the font atlas with gl::NEAREST filtering to prevent any possiblility of GUI blurriness due to sampling interpolation
            let tex_params = [
                (gl::TEXTURE_WRAP_S, gl::REPEAT),
                (gl::TEXTURE_WRAP_T, gl::REPEAT),
                (gl::TEXTURE_MIN_FILTER, gl::NEAREST),
                (gl::TEXTURE_MAG_FILTER, gl::NEAREST)
            ];

            let mut tex = 0;
            gl::GenTextures(1, &mut tex);
            gl::BindTexture(gl::TEXTURE_2D, tex);            
            glutil::apply_texture_parameters(gl::TEXTURE_2D, &tex_params);
            gl::TexImage2D(gl::TEXTURE_2D, 0, gl::RGBA as GLsizei, font_atlas.width as GLsizei, font_atlas.height as GLsizei, 0, gl::RGBA, gl::UNSIGNED_BYTE, font_atlas.data.as_ptr() as _);
            atlas.tex_id = TextureId::new(tex as usize);
        }
        FontAtlasRefMut::Shared(_) => {
            panic!("Not dealing with this case.");
        }
    }

    
    let mut connection: Option<sqlite::Connection> = None; //Connection to the database    
    let mut tags = Vec::new(); //Fetch tags from database
    let mut selected_image_tags = vec![false; tags.len()];          //An array of which tags are selected for the selected image. Indexed by alphabetical order

    selected_image_tags = vec![false; tags.len()];


    let mut open_images: Vec<OpenImage> = vec![];                   //Array of structs containing data for each currently open image in the program
    let mut new_tag_buffer = String::with_capacity(256);   //Buffer for the tag name input box
    let mut selected_tag = 0;                                       //Index into tags filter dropdown
    let mut time_selected = 0.0;                                    //Value of elapsed_time when the currently selected image was selected
    let mut pics_per_row = 3;                                       //Number of pictures in a row
    let mut auto_scroll = false;                                    //Auto-scroll flag
    let mut auto_scroll_speed = 200.0;
    
    let mut selected_index = None;                                  //Index into open_images of which image is currently selected or None
    
    //Set up the thread for loading the image data from disk
    let (path_tx, path_rx) = mpsc::channel();                   //Channel for sending paths to the loader thread
    let (openimage_tx, openimage_rx) = mpsc::channel();         //Channel for sending image data back to the main thread
    let mut loader_thread = LoaderThread::new(path_tx);         //Client-side tracking of loader thread data
    thread::spawn(move || {
        //recv() is a blocking function, so this is an infinite loop
        while let Ok(path) = path_rx.recv() {
            let image_data = glutil::image_data_from_path(&path, glutil::ColorSpace::Gamma);
            send_or_error(&openimage_tx, (image_data, path));
        }
    });

    //Struct of timing data
    let mut frame_timer = ozy::structs::FrameTimer::new();

    //Main application loop
    while !window.should_close() {
        frame_timer.update();

        //This section is mostly just passing window events into Dear Imgui
        let imgui_io = imgui_context.io_mut();      //Getting reference to the io data struct
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
                    let idx = button as usize - MouseButton::Button1 as usize;

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
                WindowEvent::Char(c) => { imgui_io.add_input_character(c); }
                WindowEvent::FileDrop(file_paths) => {
                    for path in file_paths {
                        let s = String::from(path.to_str().unwrap());
                        loader_thread.queue_image(s);
                    }
                }
                _ => { println!("Unhandled event: {:?}", event); }
            }
        }

        //Set modifier keys
        imgui_io.key_ctrl = imgui_io.keys_down[Key::LeftControl as usize] || imgui_io.keys_down[Key::RightControl as usize];
        imgui_io.key_shift = imgui_io.keys_down[Key::LeftShift as usize] || imgui_io.keys_down[Key::RightShift as usize];
        imgui_io.key_alt = imgui_io.keys_down[Key::LeftAlt as usize] || imgui_io.keys_down[Key::RightAlt as usize];        

        //Begin Imgui drawing
        let imgui_ui = imgui_context.frame();

        //Receive an image from the image loading thread
        if let Ok((image, path)) = openimage_rx.try_recv() {
            //Create the open image struct
            let mut open_image = OpenImage::from_imagedata(image, path);

            //Copy this image into IMAGE_DIRECTORY if it isn't already there
            let new_path = format!("{}/{}", image_directory, open_image.name);
            if !Path::new(&new_path).exists() {
                //Copy the file to the new path
                if let Err(e) = fs::copy(&open_image.orignal_path, &new_path) {
                    println!("Error migrating {} to {}: {}", open_image.orignal_path, image_directory, e);
                }
            }
            
            if let Some(con) = &connection {
                //Insert this image into the database if it doesn't already exist
                while let Err(e) = con.execute(format!(
                    "
                    INSERT OR IGNORE INTO images (path) VALUES (\"{}\");
                    ", open_image.name
                )) {
                    println!("{}", e);
                }

                //Retrieve all tags for this image from the DB
                let mut tag_statement = con.prepare("
                    SELECT name FROM tags
                    JOIN
                    (SELECT tag_id FROM image_tags
                    WHERE image_tags.image_id = (
                            SELECT id FROM images WHERE path=?
                        ))
                    WHERE id=tag_id ORDER BY name;
                ").unwrap();
                tag_statement.bind(1, &*open_image.name).unwrap();

                //Push each tag into the image's tags array
                while let State::Row = tag_statement.next().unwrap() {
                    open_image.tags.push(tag_statement.read::<String>(0).unwrap().into());
                }
            }

            open_images.push(open_image);
            loader_thread.images_in_flight -= 1;
        }

        //Draw main window where images are displayed
        if let Some(token) = imgui::Window::new("uwu_db")
                            .position([0.0, 0.0], Condition::Always)
                            .size([window_size.x as f32, window_size.y as f32], Condition::Always)
                            .resizable(false)
                            .collapsible(false)
                            .title_bar(false)
                            .horizontal_scrollbar(true)
                            .menu_bar(true)
                            .begin(&imgui_ui) {

            //Do auto scrolling
            if auto_scroll {
                let dist = auto_scroll_speed * frame_timer.delta_time;
                let new_scroll = imgui_ui.scroll_y() + dist;
                if new_scroll >= imgui_ui.scroll_max_y() {
                    imgui_ui.set_scroll_y(0.0);
                } else {
                    imgui_ui.set_scroll_y(imgui_ui.scroll_y() + dist);
                }
            }

            let side_panel_width = 200.0;
            imgui_ui.columns(2, "main_columns", false);
            imgui_ui.set_current_column_width(window_size.x as f32 - side_panel_width);

            if let Some(menu_token) = imgui_ui.begin_menu_bar() {
                if let Some(file_token) = imgui_ui.begin_menu("File") {
                    if MenuItem::new("New database").build(&imgui_ui) {
                        if let Some(dir_path) = tfd::select_folder_dialog("Image location", ".") {
                            tags.clear();
                            image_directory = dir_path;
                            connection =  {
                                let con = sqlite::open(format!("{}/images.db", image_directory)).unwrap();
                                
                                //Initialize the tables
                                con.execute(
                                    "
                                    CREATE TABLE images (id INTEGER, path STRING NOT NULL UNIQUE, PRIMARY KEY (id));
                                    CREATE TABLE tags (id INTEGER, name STRING NOT NULL UNIQUE, PRIMARY KEY (id));
                                    CREATE TABLE image_tags (image_id INTEGER, tag_id INTEGER);
                                    "
                                ).unwrap();

                                Some(con)
                            };
                        }
                    }

                    if MenuItem::new("Open database").build(&imgui_ui) {
                        if let Some(db_path) = tfd::open_file_dialog("Open database", "", Some((&["*.db"], "database"))) {
                            image_directory = String::from(Path::new(&db_path).parent().unwrap().to_str().unwrap());

                            connection = if !Path::new(&db_path).exists() {
                                let con = sqlite::open(db_path).unwrap();
                                
                                //Initialize the tables
                                con.execute(
                                    "
                                    CREATE TABLE images (id INTEGER, path STRING NOT NULL UNIQUE, PRIMARY KEY (id));
                                    CREATE TABLE tags (id INTEGER, name STRING NOT NULL UNIQUE, PRIMARY KEY (id));
                                    CREATE TABLE image_tags (image_id INTEGER, tag_id INTEGER);
                                    "
                                ).unwrap();
                                
                                Some(con)
                            } else { Some(sqlite::open(db_path).unwrap()) };

                            //Fetch tags from database
                            tags = match &connection {
                                Some(con) => {
                                    let mut tag_statement = con.prepare(
                                        "
                                        SELECT name FROM tags ORDER BY name;
                                        "
                                    ).unwrap();

                                    //Convert to imgui::ImString
                                    let mut ts = Vec::new();
                                    while let State::Row = tag_statement.next().unwrap() {
                                        let the_string = tag_statement.read::<String>(0).unwrap();
                                        ts.push(the_string.into());
                                    }
                                    ts
                                }
                                None => {
                                    Vec::new()
                                }
                            };

                            selected_image_tags = vec![false; tags.len()];
                        }
                    }

                    if MenuItem::new("Exit").build(&imgui_ui) {
                        window.set_should_close(true);
                    }

                    file_token.end();
                }

                menu_token.end();
            }

            //Drawing each open_image as an imgui::ImageButton
            let max_width = (window_size.x as f32 - side_panel_width) / pics_per_row as f32 - 24.0;
            for i in 0..open_images.len() {
                let im = &open_images[i];
                let factor = max_width as f32 / im.width as f32;

                //Color the selected image
                let tint_color = match selected_index {
                    Some(idx) => {
                        if i == idx {
                            let time = frame_timer.elapsed_time - time_selected;
                            let func = 0.5 * f32::cos(6.0 * time) + 0.5;
                            [1.0, 1.0, func, 1.0]
                        } else { 
                            [1.0, 1.0, 1.0, 1.0]
                        }
                    }
                    None => {
                        [1.0, 1.0, 1.0, 1.0]
                    }
                };

                if ImageButton::new(TextureId::new(im.gl_name as usize),
									[im.width as f32 * factor, im.height as f32 * factor])
									.tint_col(tint_color)
									.build(&imgui_ui) {
                    selected_index = Some(i);
                    time_selected = frame_timer.elapsed_time;

                    //Compute selected_image_tags
                    recompute_selected_tags(&mut selected_image_tags, &tags, &im.tags);
                }
                imgui_ui.same_line();
                if i as u32 % pics_per_row == pics_per_row - 1 {
                    imgui_ui.new_line();
                }
            }            

            //Specify side panel
            const Y_PADDING: f32 = 27.0;
            imgui_ui.next_column();
            imgui_ui.set_cursor_pos([imgui_ui.cursor_pos()[0], imgui_ui.scroll_y() + Y_PADDING]);
            imgui_ui.set_current_column_width(side_panel_width);
            
            //Button to spawn a file(s) selection dialogue for loading images
            if imgui_ui.button_with_size("Open image(s)", [0.0, 32.0]) {
                if let Some(image_paths) = tfd::open_file_dialog_multi("Open image", &image_directory, Some((&["*.png", "*.jpg"], ".png, .jpg"))) {
                    for path in image_paths {
                        loader_thread.queue_image(path);
                    }
                }
            }

            if imgui_ui.button_with_size("Load tagless images", [0.0, 32.0]) {
                match &connection {
                    Some(con) => {
                        clear_open_images(&mut open_images, &mut selected_index);
                        let mut statement = con.prepare("
                            SELECT path FROM images
                            JOIN
                                (SELECT id AS im_id FROM images
                                EXCEPT
                                SELECT image_id from image_tags)
                            WHERE id=im_id ORDER BY random();
                        ").unwrap();

                        let tagless_loaded = 200;
                        let mut count = 0;
                        while let State::Row = statement.next().unwrap() {
                            if count < tagless_loaded {
                                let path = format!("{}/{}", image_directory, statement.read::<String>(0).unwrap());
                                loader_thread.queue_image(path);
                                count += 1;
                            }
                        }
                    }
                    None => {
                        tfd::message_box_ok("No loaded database", "One cannot load images from a database that is not there\n-Kanye", MessageBoxIcon::Error);
                    }
                }

                
            }
                                
            if imgui_ui.button_with_size("Close open images", [0.0, 32.0]) {
                clear_open_images(&mut open_images, &mut selected_index);
            }

            //Slider for selecting how many images are in a row
            imgui_ui.text("Images per row");
            imgui_ui.set_next_item_width(side_panel_width - 50.0);
            imgui::Slider::new("###Images per row", 1, 16).build(&imgui_ui, &mut pics_per_row);

            imgui_ui.text("Active tag");
            imgui_ui.set_next_item_width(side_panel_width - 50.0);            
            if loader_thread.images_in_flight == 0 && imgui_ui.combo_simple_string("###Active tag", &mut selected_tag, imstr_ref_array(&tags).as_slice()) {
                clear_open_images(&mut open_images, &mut selected_index);
                imgui_ui.set_scroll_y(0.0);

                if let Some(con) = &connection {
                    let mut statement = con.prepare("
                        SELECT path FROM images
                        JOIN
                        (SELECT image_id FROM image_tags
                        WHERE image_tags.tag_id = (
                                SELECT id FROM tags WHERE name=?
                            ))
                        WHERE id=image_id ORDER BY random();
                    ").unwrap();
                    statement.bind(1, tags[selected_tag].to_str()).unwrap();
                    while let State::Row = statement.next().unwrap() {
                        let s = statement.read::<String>(0).unwrap();
                        let path = format!("{}/{}", image_directory, s);
                        loader_thread.queue_image(path);
                    }
                }
            }

            imgui_ui.text("Scroll speed");
            imgui_ui.set_next_item_width(side_panel_width - 50.0);
            imgui::Slider::new("###Scroll speed", 150.0, 750.0).build(&imgui_ui, &mut auto_scroll_speed);
            if imgui_ui.button_with_size("Toggle auto-scrolling", [0.0, 32.0]) {
                auto_scroll = !auto_scroll;
            }

            if let Some(_) = selected_index {
                
                if let Some(token) = imgui::Window::new("uwu_db")
                                     .position([0.0, 0.0], Condition::Always)
                                     .size([window_size.x as f32, window_size.y as f32], Condition::Always)
                                     .resizable(true)
                                     .collapsible(false)
                                     .title_bar(false)
                                     .begin(&imgui_ui) {
                    
                    //                    
                    
                    token.end()
                }
            }

            token.end();
        }

        //Image control panel window
        if let Some(image_idx) = selected_index {
            //Close this control panel window
            fn close_window(selected_image: &mut Option<usize>) {
                *selected_image = None;
            }

            //Removes image from list of currently displayed images
            fn close_image(to_remove: &mut bool, selected_image: &mut Option<usize>) {
                *to_remove = true;                    
                close_window(selected_image);
            }

            fn delete_image(path: &str) {
                if let Err(e) = fs::remove_file(path) {
                    println!("Error deleting {}: {}", path, e);
                }                
            }

            let im = &mut open_images[image_idx];       //Get mutable reference to the selected image
            let mut removing_this = false;            //Flag for if we want to close this image

            //Create control panel for manipulating the selected image
            if let Some(token) = imgui::Window::new(&format!("{}###control_panel", im.orignal_path))
                                 .collapsible(false)
                                 .position(imgui_ui.cursor_pos(), Condition::Once)   //We want it to spawn roughly in the middle of the screen the first time it's opened
                                 .begin(&imgui_ui) {

                //We want the control panel to close as soon as it loses focus
                if !imgui_ui.is_window_focused_with_flags(WindowFocusedFlags::CHILD_WINDOWS) {
                    close_window(&mut selected_index);
                }

                //Button to remove image from the current set
                if imgui_ui.button_with_size("Close this image", [0.0, 32.0]) {
                    close_image(&mut removing_this, &mut selected_index);
                }
                imgui_ui.same_line();

                //Create button for completely deleting image
                if imgui_ui.button_with_size("Delete this image", [0.0, 32.0]) {

                    //Pop up confirmation dialogue for image deletion
                    if let YesNo::Yes = tfd::message_box_yes_no("Delete this image", &format!("You are about to permanently delete\n{}\nProceed?", im.name), MessageBoxIcon::Warning, YesNo::No) {
                        //Delete the image and its relationships from the database
                        if let Some(con) = &connection {
                            con.execute(format!("
                                DELETE FROM image_tags WHERE image_id=(SELECT id FROM images WHERE path=\"{}\");
                                DELETE FROM images WHERE path=\"{}\";
                            ", im.name, im.name)).unwrap();
                        }
                        
                        close_image(&mut removing_this, &mut selected_index);
                        delete_image(&im.orignal_path);

                        let std_path = format!("{}/{}", image_directory, im.name);
                        if Path::new(&std_path) != Path::new(&im.orignal_path) {
                            delete_image(&std_path);
                        }
                    }
                }
                imgui_ui.same_line();

                if imgui_ui.button_with_size("Copy path to clipboard", [0.0, 32.0]) {
                    imgui_ui.set_clipboard_text(&im.orignal_path);
                }

                imgui_ui.separator();

                //Create a text input field for entering tag names into
                imgui::InputText::new(&imgui_ui, "Create a new tag", &mut new_tag_buffer).build();
                if new_tag_buffer.len() > 0 && imgui_ui.button_with_size("Create tag and apply to image", [0.0, 32.0]) {
                    let new_tag = new_tag_buffer.clone();       //Make a copy of the text in the input field
                    new_tag_buffer.clear();                     //Clear the input field                    

                    //Do nothing if this tag already exists
                    let new_tag = new_tag.into();
                    if !tags.contains(&new_tag) {
                        match &connection {
                            Some(con) => {
                                //Insert tags into appropriate arrays
                                insert_tag(&mut tags, &new_tag);
                                insert_tag(&mut im.tags, &new_tag);
            
                                selected_image_tags.push(false);
                                recompute_selected_tags(&mut selected_image_tags, &tags, &im.tags);

                                //Do SQL
                                con.execute(format!(
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
                            None => { tfd::message_box_ok("Saving with no db", "You need to open a database before you can do this", MessageBoxIcon::Error); }
                        }
                    }
                }
                imgui_ui.separator();

                imgui_ui.text("Tag selection:");

                //Drawing a checkbox per registered tag
                let tags_per_column = 20;
                let column_count = f32::ceil(tags.len() as f32 / tags_per_column as f32 + 1.0) as u32;
                imgui_ui.columns(column_count as i32, "Tag selection", false);                
                let mut to_remove = None;
                for i in 0..tags.len() {
                    if imgui_ui.checkbox(&tags[i], &mut selected_image_tags[i]) {
                        if selected_image_tags[i] {
                            match &connection {
                                Some(con) => {
                                    con.execute(format!(
                                        "
                                        INSERT OR IGNORE INTO image_tags VALUES (
                                            (SELECT id FROM images WHERE path=\"{}\")
                                        ,   (SELECT id FROM tags WHERE name=\"{}\")
                                        );
                                        ", im.name, tags[i].to_str()
                                    )).unwrap();
                                }
                                None => { tfd::message_box_ok("Saving with no db", "You need to open a database before you can do this", MessageBoxIcon::Error); }
                            }
                            insert_tag(&mut im.tags, &tags[i]);
                        } else {
                            let image_tag_index = im.tags.binary_search(&tags[i]).unwrap();
                            to_remove = Some(image_tag_index);
                        }
                    }

                    if i % tags_per_column == tags_per_column - 1 {
                        imgui_ui.next_column();
                    }
                }

                //If we're removing a tag from an image
                if let Some(idx) = to_remove {
                    //Delete the relationship in the database                    
                    if let Some(con) = &connection {
                        con.execute(format!("
                            DELETE FROM image_tags WHERE image_id=(
                                SELECT id FROM images WHERE path=\"{}\"
                            ) AND tag_id=(
                                SELECT id FROM tags WHERE name=\"{}\"
                            )
                        ", im.name, im.tags[idx].to_str())).unwrap();
                        im.tags.remove(idx);
                    }
                }

                token.end();
            }

            //Removing the selected image from the list
            if removing_this {
                open_images.remove(image_idx);
            }
        }

        //Rendering Dear IMGUI
        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT);            
            gl::Viewport(0, 0, window_size.x as GLint, window_size.y as GLint);
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
                                    gl::BindVertexArray(imgui_vao.vao);
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
                        let bufs = [imgui_vao.vbo, imgui_vao.ebo];
                        gl::DeleteBuffers(2, &bufs[0]);
                        gl::DeleteVertexArrays(1, &imgui_vao.vao);
                    }
                }
            }
        }

        //Present the drawn frame before returning to the beginning of the loop
        window.swap_buffers();

        //glfw.wait_events_timeout(0.1);
        glfw.poll_events();
    }
}
