use gl::types::*;
use imgui::ImString;
use ozy::glutil;
use ozy::structs::ImageData;
use std::sync::mpsc::Sender;

use crate::*;

pub struct OpenImage {
    pub name: String,
    pub tags: Vec<ImString>,
    pub gl_name: GLuint,
    pub width: usize,
    pub height: usize
}

impl OpenImage {
    pub fn from_path(path: String) -> Self {
        println!("Trying to load: {}", path);
        let image_data = glutil::image_data_from_path(&path, glutil::ColorSpace::Gamma);
        let height = image_data.height;
        let width = image_data.width;
        let gl_name = unsafe { glutil::load_texture_from_data(image_data, &DEFAULT_TEX_PARAMS) };
        println!("Loaded successfully.");
        OpenImage {
            name: path,
            tags: Vec::new(),
            gl_name,
            width: width as usize,
            height: height as usize
        }
    }

    pub fn from_imagedata(image_data: ImageData, path: String) -> Self {
        let height = image_data.height;
        let width = image_data.width;
        let gl_name = unsafe { glutil::load_texture_from_data(image_data, &DEFAULT_TEX_PARAMS) };

        OpenImage {
            name: path,
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