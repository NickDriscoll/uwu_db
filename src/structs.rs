use gl::types::*;
use imgui::ImString;
use ozy::glutil;
use ozy::structs::ImageData;
use std::sync::mpsc::Sender;

use crate::*;

//Stores all the state required for an image the program has loaded
pub struct OpenImage {
    pub name: String,				//Just the filename without extension
    pub orignal_path: String,       //The original path the image was opened from
    pub tags: Vec<ImString>,		//Array of tags
    pub gl_name: GLuint,			//GL texture
    pub width: usize,				//Image width in pixels
    pub height: usize				//Image height in pixels
}

impl OpenImage {
    pub fn from_imagedata(image_data: ImageData, path: String) -> Self {
        let height = image_data.height;
        let width = image_data.width;
        let gl_name = unsafe { glutil::load_texture_from_data(image_data, &DEFAULT_TEX_PARAMS) };
        
        let name = {
            let mut last_slash_index = 0;
            let mut current_index = 0;
            for c in path.chars() {
                if c == '\\' || c == '/' {
                    last_slash_index = current_index;
                }
                current_index += 1;
            }

            String::from(path.split_at(last_slash_index + 1).1)
        };

        OpenImage {
            name,
            orignal_path: path,
            tags: Vec::new(),
            gl_name,
            width: width as usize,
            height: height as usize 
        }
    }
}

impl Drop for OpenImage {
    fn drop(&mut self) {
        unsafe { gl::DeleteTextures(1, &mut self.gl_name); }
    }
}

//Represents the current state of the image loading thread
pub struct LoaderThread {
    pub images_in_flight: usize,
    pub sender: Sender<String>
}

impl LoaderThread {
    pub fn new(sender: Sender<String>) -> Self {
        LoaderThread {
            images_in_flight: 0,
            sender
        }
    }

    pub fn queue_image(&mut self, path: String) {
        send_or_error(&self.sender, path);
        self.images_in_flight += 1;
    }
}