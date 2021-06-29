use gl::types::*;
use imgui::ImStr;
use ozy::glutil;
use ozy::structs::ImageData;
use crate::DEFAULT_TEX_PARAMS;

pub struct OpenImage<'a> {
    pub name: String,
    pub tags: Vec<&'a ImStr>,
    pub gl_name: GLuint,
    pub width: usize,
    pub height: usize
}

impl<'a> OpenImage<'a> {
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

impl<'a> Drop for OpenImage<'a> {
    fn drop(&mut self) {
        unsafe { gl::DeleteTextures(1, &mut self.gl_name); }
    }
}