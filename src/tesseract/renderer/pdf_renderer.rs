use num_traits::ToPrimitive;
use std::ffi::CString;
use std::os::raw::c_char;
use std::path::Path;

pub struct PDFRenderer {
    pub raw: *mut ::capi::TessResultRenderer,
    output: *mut c_char,
    datadir: *mut c_char,
}

impl Drop for PDFRenderer {
    fn drop(&mut self) {
        unsafe {
            ::capi::TessDeleteResultRenderer(self.raw);

            if !self.output.is_null() {
                CString::from_raw(self.output);
            }
            if !self.datadir.is_null() {
                CString::from_raw(self.datadir);
            }
        }
    }
}

impl super::ResultRenderer for PDFRenderer {
    fn renderer(&self) -> *mut ::capi::TessResultRenderer {
        self.raw
    }
}

impl PDFRenderer {
    pub fn new(
        output: Option<impl AsRef<Path>>,
        datadir: impl AsRef<Path>,
        textonly: ::tesseract::TextOnly,
    ) -> Result<Self, ::leptonica::PixError> {
        let output_cstr = CString::new(
            if let Some(output) = output {
                output
                    .as_ref()
                    .to_str()
                    .ok_or(::leptonica::PixError::InvalidUtf8Path)?
                    .to_string()
            } else {
                "stdout".to_string()
            }
            .as_str(),
        )?;
        let output_cptr = output_cstr.into_raw();

        let datadir_cstr = CString::new(
            datadir
                .as_ref()
                .to_str()
                .ok_or(::leptonica::PixError::InvalidUtf8Path)?,
        )?;
        let datadir_cptr = datadir_cstr.into_raw();

        Ok(Self {
            raw: unsafe {
                ::capi::TessPDFRendererCreate(output_cptr, datadir_cptr, textonly.to_i32().unwrap())
            },
            output: output_cptr,
            datadir: datadir_cptr,
        })
    }
}
