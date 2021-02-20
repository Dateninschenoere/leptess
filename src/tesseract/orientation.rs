use std::os::raw::c_float;

use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

#[derive(FromPrimitive, Debug, PartialEq)]
pub enum Orientation {
    PageUp,
    PageDown,
    PageLeft,
    PageRight,
}

#[derive(FromPrimitive, Debug, PartialEq)]
pub enum WritingDirection {
    LeftToRight,
    RightToLeft,
    TopToBottom,
}

#[derive(FromPrimitive, Debug, PartialEq)]
pub enum TextlineOrder {
    LeftToRight,
    RightToLeft,
    TopToBottom,
}

#[derive(Debug)]
pub struct PageOrientation {
    pub orientation: Orientation,
    pub writing_direction: WritingDirection,
    pub textline_order: TextlineOrder,
    pub deskew_angle: c_float,
}

impl PageOrientation {
    pub fn from_c(
        orientation: ::capi::TessOrientation,
        writing_direction: ::capi::TessWritingDirection,
        textline_order: ::capi::TessTextlineOrder,
        deskew_angle: c_float,
    ) -> Self {
        Self {
            orientation: Orientation::from_u32(orientation).unwrap(),
            writing_direction: WritingDirection::from_u32(writing_direction).unwrap(),
            textline_order: TextlineOrder::from_u32(textline_order).unwrap(),
            deskew_angle,
        }
    }
}
