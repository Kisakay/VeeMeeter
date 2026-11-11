//! Mimi, la mascotte, dans la fenêtre. PNG pré-rendus depuis `branding/mascot`
//! (`branding/src/build.sh`), décodés une fois au démarrage.

use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};

#[derive(Clone, Copy)]
pub enum Mood {
    Wave,
    Happy,
    Sleepy,
    Surprised,
}

pub struct Mimi {
    wave: TextureHandle,
    happy: TextureHandle,
    sleepy: TextureHandle,
    surprised: TextureHandle,
}

impl Mimi {
    pub fn load(ctx: &egui::Context) -> Mimi {
        let t = |name: &str, bytes: &[u8]| ctx.load_texture(name, decode(bytes), TextureOptions::LINEAR);
        Mimi {
            wave: t("mimi-wave", include_bytes!("../../assets/gui/mimi-wave.png")),
            happy: t("mimi-happy", include_bytes!("../../assets/gui/mimi-happy.png")),
            sleepy: t("mimi-sleepy", include_bytes!("../../assets/gui/mimi-sleepy.png")),
            surprised: t("mimi-surprised", include_bytes!("../../assets/gui/mimi-surprised.png")),
        }
    }

    pub fn image(&self, mood: Mood, size: f32) -> egui::Image<'static> {
        let tex = match mood {
            Mood::Wave => &self.wave,
            Mood::Happy => &self.happy,
            Mood::Sleepy => &self.sleepy,
            Mood::Surprised => &self.surprised,
        };
        egui::Image::new((tex.id(), egui::vec2(size, size)))
    }
}

/// PNG embarqué → image RGBA (les fichiers sont générés par resvg : RGBA 8 bits).
fn decode(bytes: &[u8]) -> ColorImage {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::ALPHA);
    let mut reader = decoder.read_info().expect("embedded PNG is valid");
    let mut buf = vec![0; reader.output_buffer_size().expect("PNG size fits in memory")];
    let info = reader.next_frame(&mut buf).expect("embedded PNG decodes");
    let size = [info.width as usize, info.height as usize];
    ColorImage::from_rgba_unmultiplied(size, &buf[..info.buffer_size()])
}

#[cfg(test)]
mod tests {
    #[test]
    fn embedded_mascots_decode() {
        for bytes in
            [&include_bytes!("../../assets/gui/mimi-wave.png")[..], include_bytes!("../../assets/gui/mimi-sleepy.png")]
        {
            let img = super::decode(bytes);
            assert_eq!(img.size, [192, 192]);
        }
    }
}
