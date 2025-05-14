use hord3::defaults::default_rendering::vectorinator_binned::textures::rgb_to_argb;

pub mod number_config;
pub mod image_title_desc;
pub mod list_choice;
pub mod editor_gui_elements;
pub mod title_desc;
pub mod centered_title;
pub mod title_desc_image;


pub const DEFAULT_BACKGROUND_COLOR:u32 = rgb_to_argb((125, 125, 125));
pub const DEFAULT_CONTENT_BACKGROUND_COLOR:u32 = rgb_to_argb((50, 50, 50));
pub const DEFAULT_REACT_COLOR:u32 = rgb_to_argb((200,200,200));
pub const DEFAULT_HOVER_COLOR:u32 = rgb_to_argb((160,160,160));
pub const DEFAULT_FONT_SIZE:f32 = 25.0;
pub const DEFAULT_LINE_HEIGHT:f32 = 30.0;

