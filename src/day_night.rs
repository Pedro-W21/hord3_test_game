use hord3::{defaults::default_rendering::vectorinator_binned::triangles::{collux_f32_a_u8, collux_u8_a_f32}, horde::geometry::vec3d::Vec3Df};

pub struct DayNight {
    daylight_color:(u8, u8, u8),
    sunset_color:(u8,u8,u8),
    night_color:(u8,u8,u8),
    noon_light_dir:Vec3Df,
    sunset_light_dir:Vec3Df,
    night_light_dir:Vec3Df,
    current_tick:usize,
    half_time:usize,
}

impl DayNight {
    pub fn new(daylight_color:(u8, u8, u8), sunset_color:(u8, u8, u8), night_color:(u8,u8,u8), noon_light_dir:Vec3Df, sunset_light_dir:Vec3Df, night_light_dir:Vec3Df,  half_time:usize) -> Self {
        Self { daylight_color, sunset_color, night_color, noon_light_dir, sunset_light_dir, night_light_dir, current_tick:0, half_time  }
    }
    pub fn get_next_color(&mut self) -> ((u8,u8,u8), Vec3Df, bool) {
        let mut color = (0, 0, 0);
        let mut vector = Vec3Df::all_ones().normalise();
        let mut is_night = false;
        if self.current_tick < self.half_time/2 {
            let daylight_factor = ((self.half_time/2 - self.current_tick) as f32)/(self.half_time as f32/2.0);
            let sunset_factor = 1.0 - daylight_factor;
            let daylight = collux_u8_a_f32(self.daylight_color);
            let sunset = collux_u8_a_f32(self.sunset_color);
            color = collux_f32_a_u8((daylight_factor * daylight.0 + sunset_factor * sunset.0, daylight_factor * daylight.1 + sunset_factor * sunset.1, daylight_factor * daylight.2 + sunset_factor * sunset.2));
            vector = (self.noon_light_dir.normalise() * daylight_factor + self.sunset_light_dir.normalise() * sunset_factor).normalise();
        }
        else if self.current_tick < self.half_time {
            let daylight_factor = ((self.half_time/2 - (self.current_tick - self.half_time/2)) as f32)/(self.half_time as f32/2.0);
            let sunset_factor = 1.0 - daylight_factor;
            let daylight = collux_u8_a_f32(self.sunset_color);
            let sunset = collux_u8_a_f32(self.night_color);
            color = collux_f32_a_u8((daylight_factor * daylight.0 + sunset_factor * sunset.0, daylight_factor * daylight.1 + sunset_factor * sunset.1, daylight_factor * daylight.2 + sunset_factor * sunset.2));
            vector = (self.sunset_light_dir.normalise() * daylight_factor + self.night_light_dir.normalise() * sunset_factor).normalise();
            is_night = true;
        }
        else if self.current_tick < self.half_time + self.half_time/2 {
            let daylight_factor = ((self.half_time/2 - (self.current_tick - self.half_time)) as f32)/(self.half_time as f32/2.0);
            let sunset_factor = 1.0 - daylight_factor;
            let daylight = collux_u8_a_f32(self.night_color);
            let sunset = collux_u8_a_f32(self.sunset_color);
            color = collux_f32_a_u8((daylight_factor * daylight.0 + sunset_factor * sunset.0, daylight_factor * daylight.1 + sunset_factor * sunset.1, daylight_factor * daylight.2 + sunset_factor * sunset.2));
            vector = (self.night_light_dir.normalise() * daylight_factor + self.sunset_light_dir.normalise() * (-sunset_factor)).normalise();
            is_night = true;
        }
        else {
            let daylight_factor = ((self.half_time/2 - (self.current_tick - self.half_time - self.half_time/2)) as f32)/(self.half_time as f32/2.0);
            let sunset_factor = 1.0 - daylight_factor;
            let daylight = collux_u8_a_f32(self.sunset_color);
            let sunset = collux_u8_a_f32(self.daylight_color);
            color = collux_f32_a_u8((daylight_factor * daylight.0 + sunset_factor * sunset.0, daylight_factor * daylight.1 + sunset_factor * sunset.1, daylight_factor * daylight.2 + sunset_factor * sunset.2));
            vector = (-self.sunset_light_dir.normalise() * daylight_factor + self.noon_light_dir.normalise() * sunset_factor).normalise();
            
        }
        self.current_tick += 1;
        if self.current_tick >= self.half_time*2{
            self.current_tick = 0;
        }
        (color, vector, is_night)
    }
}