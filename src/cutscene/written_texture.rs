use cosmic_text::{Attrs, Buffer, Color, Family, Font, FontSystem, Metrics, SwashCache};
use hord3::defaults::default_rendering::vectorinator::textures::{argb_to_rgb, rgb_to_argb, rgbu_to_rgbf};

pub fn get_written_texture_buffer(text:String, metrics:Metrics, font:String, texture_buffer:Vec<u32>, texture_width:usize, texture_height:usize, text_color:Color, start_of_text:(i32, i32)) -> Vec<u32> {
    let mut final_buffer = texture_buffer.clone();
    let mut font_system = FontSystem::new();
    let mut cache = SwashCache::new();
    let mut buffer = Buffer::new(&mut font_system, metrics);
    let mut buffer = buffer.borrow_with(&mut font_system);
    buffer.set_size(Some(texture_width as f32), Some(texture_height as f32));
    buffer.shape_until_scroll(true);

    let attrs = Attrs::new().family(Family::Cursive);
    //println!("{}", text.clone());
    buffer.set_text(&text.trim(), attrs, cosmic_text::Shaping::Advanced);
    
    buffer.draw(&mut cache, text_color, |x,y,width,height, color| {
        //final_buffer[(x + start_of_content.x as i32) as usize + ((y + start_of_content.y as i32) * outside_dims.x) as usize] = rgb_to_argb((color.r(), color.g(), color.b()));
        let buffer_pos = (x + start_of_text.0 as i32) as usize + ((y + start_of_text.1 as i32) * texture_width as i32) as usize;
        if buffer_pos < final_buffer.len() {
            let rgba_color = {
                let r0 = color.r() as f32/255.0;
                let g0 = color.g() as f32/255.0;
                let b0 = color.b() as f32/255.0;
                let a0 = color.a() as f32/255.0;
                let (r1, g1, b1) = rgbu_to_rgbf(argb_to_rgb(final_buffer[buffer_pos]));
                let a1 = 1.0;
                let a01 = (1.0 - a0) * a1 + a0;
                let r01 = ((1.0 - a0) * a1 * r1 + a0 * r0)/a01;
                let g01 = ((1.0 - a0) * a1 * g1 + a0 * g0)/a01;
                let b01 = ((1.0 - a0) * a1 * b1 + a0 * b0)/a01;
                rgb_to_argb(((r01 * 255.0) as u8, (g01 * 255.0) as u8, (b01 * 255.0) as u8))
            };
            for yc in y..y + height as i32 {
                for xc in x..x + width as i32 {
                    let index = (xc + start_of_text.0 as i32) as usize + ((yc + start_of_text.1 as i32) * texture_width as i32) as usize;
                    if index < final_buffer.len() {
                        final_buffer[index] = rgba_color;
                    }
                    
                }
            }
        }
        
        
        //println!("{} {} {} {}", x, y, width, height);
        
    });

    final_buffer
} 