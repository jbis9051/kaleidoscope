use std::ffi::{c_char, c_float, CStr, CString};
use serde::Serialize;

#[repr(C)]
struct OCRResultFFI {
    pub text: *const c_char,
    pub origin_x: c_float,
    pub origin_y: c_float,
    pub size_width: c_float,
    pub size_height: c_float,
}

type c_size_t = usize;

extern "C" {
    fn OCRResult_cleanup(result: *const OCRResultFFI, count: c_size_t);
    
    fn perform_ocr(image_path: *const c_char, count: *mut c_size_t) -> *const OCRResultFFI;
}

impl OCRResultFFI {
    fn to_ocr_result(&self) -> OCRResult {
        // SAFETY: OCRResult's text field is defined as a valid C string
        let text = unsafe { CStr::from_ptr(self.text) }.to_owned().into_string().expect("ocr text");
        OCRResult {
            text,
            origin_x: self.origin_x as f32,
            origin_y: self.origin_y as f32,
            size_width: self.size_width as f32,
            size_height: self.size_height as f32,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct OCRResult {
    pub text: String,
    pub origin_x: f32,
    pub origin_y: f32,
    pub size_width: f32,
    pub size_height: f32,
}

pub fn vision_ocr(image_path: &str) -> Vec<OCRResult> {
    let image_path_c = CString::new(image_path).expect("CString::new failed");
    let mut count: c_size_t = 0;
    // SAFETY: image_path_c is a valid C string, count is a pointer to a size_t, and perform_ocr is defined such that image_path_c won't be modified
    let result_ptr = unsafe { perform_ocr(image_path_c.as_ptr(), &mut count) };
    
    let mut results = Vec::with_capacity(count as usize);
    for i in 0..count {
        // SAFETY: result_ptr is a valid pointer to an array of OCRResultFFI, and count is the number of elements in that array
        let result = unsafe { result_ptr.add(i) };
        results.push(unsafe { (*result).to_ocr_result() });
    }
    
    // SAFETY: result_ptr is a valid pointer to an array of OCRResultFFI, and count is the number of elements in that array
    unsafe { OCRResult_cleanup(result_ptr, count) };
    
    results
}