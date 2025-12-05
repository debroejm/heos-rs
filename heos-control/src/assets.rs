macro_rules! asset {
    ($name:ident, $path:literal) => {
        pub mod $name {
            pub const BYTES: &'static [u8] = include_bytes!(concat!("../assets/", $path));

            #[inline]
            pub fn image() -> egui::Image<'static> {
                egui::Image::from_bytes(concat!("bytes://assets/", $path), BYTES)
            }
        }
    };
}

pub mod icons {
    asset!(devices, "icons/devices.png");
    asset!(queue, "icons/queue.png");
    asset!(next, "icons/next.png");
    asset!(pause, "icons/pause.png");
    asset!(play, "icons/play.png");
    asset!(prev, "icons/prev.png");
    asset!(repeat, "icons/repeat.png");
    asset!(repeat_once, "icons/repeat-once.png");
    asset!(shuffle, "icons/shuffle.png");
}