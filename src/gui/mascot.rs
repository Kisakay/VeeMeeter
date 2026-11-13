//! Mimi, la mascotte, dans la fenêtre. PNG pré-rendus depuis `branding/mascot`
//! (`branding/src/build.sh`), décodés une fois au démarrage.

use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

impl Mood {
    pub const ALL: [Mood; 4] = [Mood::Wave, Mood::Happy, Mood::Sleepy, Mood::Surprised];

    /// PNG embarqué de cette humeur.
    pub fn png(self) -> &'static [u8] {
        match self {
            Mood::Wave => include_bytes!("../../assets/gui/mimi-wave.png"),
            Mood::Happy => include_bytes!("../../assets/gui/mimi-happy.png"),
            Mood::Sleepy => include_bytes!("../../assets/gui/mimi-sleepy.png"),
            Mood::Surprised => include_bytes!("../../assets/gui/mimi-surprised.png"),
        }
    }
}

impl Mimi {
    pub fn load(ctx: &egui::Context) -> Mimi {
        let t = |name: &str, mood: Mood| ctx.load_texture(name, decode(mood.png()), TextureOptions::LINEAR);
        Mimi {
            wave: t("mimi-wave", Mood::Wave),
            happy: t("mimi-happy", Mood::Happy),
            sleepy: t("mimi-sleepy", Mood::Sleepy),
            surprised: t("mimi-surprised", Mood::Surprised),
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
    let (size, rgba) = decode_rgba(bytes);
    ColorImage::from_rgba_unmultiplied(size, &rgba)
}

/// PNG embarqué → `([largeur, hauteur], pixels RGBA non prémultipliés)`.
pub fn decode_rgba(bytes: &[u8]) -> ([usize; 2], Vec<u8>) {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::ALPHA);
    let mut reader = decoder.read_info().expect("embedded PNG is valid");
    let mut buf = vec![0; reader.output_buffer_size().expect("PNG size fits in memory")];
    let info = reader.next_frame(&mut buf).expect("embedded PNG decodes");
    buf.truncate(info.buffer_size());
    ([info.width as usize, info.height as usize], buf)
}

#[cfg(test)]
mod tests {
    #[test]
    fn embedded_mascots_decode() {
        for mood in super::Mood::ALL {
            let img = super::decode(mood.png());
            assert_eq!(img.size, [192, 192]);
        }
    }
}
