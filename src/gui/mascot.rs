//! Mimi, la mascotte. PNG pré-rendus depuis `branding/mascot`
//! (`branding/src/build.sh`), décodés en RGBA pour la fenêtre et le tray.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Mood {
    Wave,
    Happy,
    Sleepy,
    Surprised,
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
            let ([w, h], rgba) = super::decode_rgba(mood.png());
            assert_eq!((w, h), (192, 192));
            assert_eq!(rgba.len(), 192 * 192 * 4);
        }
    }
}
