//! Low level wrapper for Tesseract C API

use super::capi;
use super::leptonica;
use std::{os::raw::c_int, path::Path, sync::Arc, thread};

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

pub use capi::kMaxCredibleResolution as MAX_CREDIBLE_RESOLUTION;
pub use capi::kMinCredibleResolution as MIN_CREDIBLE_RESOLUTION;

use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::ToPrimitive;

pub mod monitor;
pub mod orientation;
pub mod renderer;

#[derive(FromPrimitive, ToPrimitive, Debug)]
pub enum PageIteratorLevel {
    Block,
    Para,
    Textline,
    Word,
    Symbol,
}

#[derive(Debug, PartialEq)]
pub struct TessInitError {
    pub code: i32,
}

impl std::error::Error for TessInitError {}

impl std::fmt::Display for TessInitError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "TessInitError{{{}}}", self.code)
    }
}

#[derive(Debug, PartialEq)]
pub struct TessApi {
    pub raw: *mut capi::TessBaseAPI,
    data_path_cptr: *mut c_char,
}

impl Drop for TessApi {
    fn drop(&mut self) {
        unsafe {
            capi::TessBaseAPIEnd(self.raw);
            capi::TessBaseAPIDelete(self.raw);

            if !self.data_path_cptr.is_null() {
                // free data_path_cptr, drop trait will take care of it
                CString::from_raw(self.data_path_cptr);
            }
        }
    }
}

impl TessApi {
    pub fn new<'a>(data_path: Option<&'a str>, lang: &'a str) -> Result<TessApi, TessInitError> {
        let data_path_cptr;
        let data_path_cstr;
        let lang = CString::new(lang).unwrap();
        match data_path {
            Some(dstr) => {
                data_path_cstr = CString::new(dstr).unwrap();
                data_path_cptr = data_path_cstr.into_raw();
            }
            None => {
                data_path_cptr = ptr::null_mut();
            }
        }

        let api = TessApi {
            raw: unsafe { capi::TessBaseAPICreate() },
            data_path_cptr,
        };

        unsafe {
            //let re = capi::TessBaseAPIInit3(api.raw, api.data_path_cptr, lang.as_ptr());
            let re = capi::TessBaseAPIInit2(api.raw, api.data_path_cptr, lang.as_ptr(), capi::TessOcrEngineMode_OEM_TESSERACT_LSTM_COMBINED);

            if re == 0 {
                Ok(api)
            } else {
                Err(TessInitError { code: re })
            }
        }
    }

    /// Provide an image for Tesseract to recognize.
    ///
    /// set_image clears all recognition results, and sets the rectangle to the full image, so it
    /// may be followed immediately by a `[Self::get_utf8_text]`, and it will automatically perform
    /// recognition.
    pub fn set_image(&mut self, img: &leptonica::Pix) {
        // "Tesseract takes its own copy of the image, so it need not persist until after Recognize"
        unsafe { capi::TessBaseAPISetImage2(self.raw, img.raw as *mut capi::Pix) }
    }

    /// Get the dimensions of the currently loaded image, or None if no image is loaded.
    ///
    /// # Example
    /// ```rust
    /// let path = std::path::Path::new("tests/di.png");
    /// let img = leptess::leptonica::pix_read(&path).unwrap();
    ///
    /// let mut tes = leptess::tesseract::TessApi::new(Some("tests/tessdata"), "eng").unwrap();
    /// tes.set_image(&img);
    ///
    /// assert_eq!(tes.get_image_dimensions(), Some((442, 852)));
    /// ```
    pub fn get_image_dimensions(&self) -> Option<(u32, u32)> {
        unsafe {
            let pix = capi::TessBaseAPIGetInputImage(self.raw);
            if pix.is_null() {
                return None;
            }

            Some(((*pix).w as u32, (*pix).h as u32))
        }
    }

    pub fn get_source_y_resolution(&mut self) -> i32 {
        unsafe { capi::TessBaseAPIGetSourceYResolution(self.raw) }
    }

    /// Override image resolution.
    /// Can be used to suppress "Warning: Invalid resolution 0 dpi." output.
    pub fn set_source_resolution(&mut self, res: i32) {
        unsafe { capi::TessBaseAPISetSourceResolution(self.raw, res) }
    }

    /// recognize set image
    /// can be used with a monitor which has to be asynchronously
    /// # Example
    /// ```rust
    /// let path = std::path::Path::new("tests/di.png");
    /// let mut tes = leptess::tesseract::TessApi::new(Some("tests/tessdata"), "eng").unwrap();
    ///
    /// tes.set_pagesegmode(leptess::capi::TessPageSegMode_PSM_AUTO_OSD);
    /// tes.recognize(None);
    /// ```
    pub fn recognize(&self, monitor: Option<Arc<monitor::Monitor>>) -> i32 {
        if let Some(m) = monitor {
            // clone the monitor, because it has to be used in two threads
            let monitor_writer = m.clone();

            // spawn a thread that runs as long as there is work to do (progress < 100)
            let monitor_thread = thread::spawn(move || loop {
                let progress = m.get_progress();
                print!("\r{:3}%", progress);

                if progress >= 100 {
                    println!();
                    break;
                }
            });

            let result = unsafe { capi::TessBaseAPIRecognize(self.raw, monitor_writer.get()) };
            monitor_thread.join().unwrap();
            result
        } else {
            unsafe { capi::TessBaseAPIRecognize(self.raw, ptr::null_mut()) }
        }
    }

    pub fn set_rectangle(&mut self, b: &leptonica::Box) {
        let v = b.get_val();
        unsafe {
            capi::TessBaseAPISetRectangle(self.raw, v.x, v.y, v.w, v.h);
        }
    }

    pub fn get_utf8_text(&self) -> Result<String, std::str::Utf8Error> {
        unsafe {
            let re: Result<String, std::str::Utf8Error>;
            let sptr = capi::TessBaseAPIGetUTF8Text(self.raw);
            match CStr::from_ptr(sptr).to_str() {
                Ok(s) => {
                    re = Ok(s.to_string());
                }
                Err(e) => {
                    re = Err(e);
                }
            }
            capi::TessDeleteText(sptr);
            re
        }
    }

    pub fn get_hocr_text(&self, page: c_int) -> Result<String, std::str::Utf8Error> {
        unsafe {
            let sptr = capi::TessBaseAPIGetHOCRText(self.raw, page);
            let re = match CStr::from_ptr(sptr).to_str() {
                Ok(s) => Ok(s.to_string()),
                Err(e) => Err(e),
            };
            capi::TessDeleteText(sptr);
            re
        }
    }

    pub fn get_alto_text(&self, page: c_int) -> Result<String, std::str::Utf8Error> {
        unsafe {
            let sptr = capi::TessBaseAPIGetAltoText(self.raw, page);
            let re = match CStr::from_ptr(sptr).to_str() {
                Ok(s) => Ok(s.to_string()),
                Err(e) => Err(e),
            };
            capi::TessDeleteText(sptr);
            re
        }
    }

    pub fn get_tsv_text(&self, page: c_int) -> Result<String, std::str::Utf8Error> {
        unsafe {
            let sptr = capi::TessBaseAPIGetTsvText(self.raw, page);
            let re = match CStr::from_ptr(sptr).to_str() {
                Ok(s) => Ok(s.to_string()),
                Err(e) => Err(e),
            };
            capi::TessDeleteText(sptr);
            re
        }
    }

    pub fn get_lstm_box_text(&self, page: c_int) -> Result<String, std::str::Utf8Error> {
        unsafe {
            let sptr = capi::TessBaseAPIGetLSTMBoxText(self.raw, page);
            let re = match CStr::from_ptr(sptr).to_str() {
                Ok(s) => Ok(s.to_string()),
                Err(e) => Err(e),
            };
            capi::TessDeleteText(sptr);
            re
        }
    }

    pub fn get_word_str_box_text(&self, page: c_int) -> Result<String, std::str::Utf8Error> {
        unsafe {
            let sptr = capi::TessBaseAPIGetWordStrBoxText(self.raw, page);
            let re = match CStr::from_ptr(sptr).to_str() {
                Ok(s) => Ok(s.to_string()),
                Err(e) => Err(e),
            };
            capi::TessDeleteText(sptr);
            re
        }
    }

    pub fn mean_text_conf(&self) -> i32 {
        unsafe { capi::TessBaseAPIMeanTextConf(self.raw) }
    }

    pub fn get_regions(&self) -> Option<leptonica::Boxa> {
        unsafe {
            let boxes = capi::TessBaseAPIGetRegions(self.raw, ptr::null_mut());
            if boxes.is_null() {
                None
            } else {
                Some(leptonica::Boxa { raw: boxes })
            }
        }
    }

    /// Get the given level kind of components (block, textline, word etc.) as a leptonica-style
    /// Boxa, in reading order.If text_only is true, then only text components are returned.
    pub fn get_component_images(
        &self,
        level: PageIteratorLevel,
        text_only: bool,
    ) -> Option<leptonica::Boxa> {
        let text_only_val: i32 = if text_only { 1 } else { 0 };
        unsafe {
            let boxes = capi::TessBaseAPIGetComponentImages(
                self.raw,
                level.to_u32().unwrap(),
                text_only_val,
                ptr::null_mut(),
                ptr::null_mut(),
            );

            if boxes.is_null() {
                None
            } else {
                Some(leptonica::Boxa { raw: boxes })
            }
        }
    }

    /// process the pages with a renderer
    ///
    /// # Example
    /// ```rust
    /// let path = std::path::Path::new("tests/di.png");
    /// let mut tes = leptess::tesseract::TessApi::new(Some("tests/tessdata"), "eng").unwrap();
    ///
    /// let pdf_renderer = leptess::tesseract::renderer::PDFRenderer::new(Some("di"), "tests/tessdata", leptess::tesseract::TextOnly::AlsoUseImage).unwrap();
    /// assert!(tes.process_pages(path, None::<&str>, 5000, pdf_renderer).unwrap());
    /// ```
    pub fn process_pages(
        &self,
        input: impl AsRef<Path>,
        retry_config_file: Option<impl AsRef<Path>>,
        timeout_ms: i32,
        renderer: impl renderer::ResultRenderer,
    ) -> Result<bool, leptonica::PixError> {
        // TODO: write macro for this
        let input_cstr = CString::new(input.as_ref().to_str().unwrap())?;
        let input_cptr = input_cstr.into_raw();

        let retry_config;
        let retry_config_ptr;
        match retry_config_file {
            Some(filename) => {
                retry_config = CString::new(filename.as_ref().to_str().unwrap())?;
                retry_config_ptr = retry_config.into_raw();
            }
            None => {
                retry_config_ptr = ptr::null_mut();
            }
        };

        Ok(unsafe {
            capi::TessBaseAPIProcessPages(
                self.raw,
                input_cptr,
                retry_config_ptr,
                timeout_ms,
                renderer.renderer(),
            ) == 1
        })
    }

    /// process the pages with a renderer
    ///
    /// # Example
    /// ```rust
    /// let path = std::path::Path::new("tests/di.png");
    /// let mut tes = leptess::tesseract::TessApi::new(Some("tests/tessdata"), "eng").unwrap();
    ///
    /// let img = leptess::leptonica::pix_read(&path).unwrap();
    /// tes.set_image(&img);
    ///
    /// tes.set_pagesegmode(leptess::capi::TessPageSegMode_PSM_AUTO_OSD);
    /// tes.recognize(None);
    /// let orientation = tes.find_orientation();
    /// assert_eq!(leptess::tesseract::orientation::Orientation::PageUp, orientation.orientation);
    /// ```
    pub fn find_orientation(&self) -> orientation::PageOrientation {
        let it = unsafe { capi::TessBaseAPIAnalyseLayout(self.raw) };
        let mut orientation: capi::TessOrientation = 0;
        let mut writing_direction: capi::TessWritingDirection = 0;
        let mut textline_order: capi::TessTextlineOrder = 0;
        let mut deskew_angle: f32 = 0.0;

        unsafe {
            capi::TessPageIteratorOrientation(
                it,
                &mut orientation,
                &mut writing_direction,
                &mut textline_order,
                &mut deskew_angle,
            )
        };

        orientation::PageOrientation::from_c(
            orientation,
            writing_direction,
            textline_order,
            deskew_angle,
        )
    }

    pub fn set_pagesegmode(&self, mode: capi::TessPageSegMode) {
        unsafe { capi::TessBaseAPISetPageSegMode(self.raw, mode) }
    }

    pub fn get_image(&self) -> *mut capi::Pix {
        unsafe { capi::TessBaseAPIGetThresholdedImage(self.raw) }
    }

    pub fn get_datadir(&self) -> *mut c_char {
        self.data_path_cptr
    }
}

#[derive(ToPrimitive, FromPrimitive, Debug)]
pub enum TextOnly {
    AlsoUseImage,
    TextOnly,
}

#[cfg(test)]
mod test {
    use crate::*;

    #[test]
    #[ignore]
    pub fn recognize() {
        let path = std::path::Path::new("tests/di.png");
        let mut tes = tesseract::TessApi::new(Some("tests/tessdata"), "eng").unwrap();

        let img = leptonica::pix_read(&path).unwrap();
        tes.set_image(&img);

        tes.set_pagesegmode(capi::TessPageSegMode_PSM_AUTO_OSD);
        tes.recognize(Some(
            std::sync::Arc::new(tesseract::monitor::Monitor::new()),
        ));
    }
}
