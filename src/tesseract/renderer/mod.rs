pub mod pdf_renderer;
pub use self::pdf_renderer::*;

pub trait ResultRenderer {
    fn renderer(&self) -> *mut ::capi::TessResultRenderer;
}
