//! Build script for graft-gui.
//!
//! On Windows, converts the default icon PNG to ICO format and embeds it
//! as the application icon using winres.

fn main() {
    #[cfg(target_os = "windows")]
    {
        use std::fs::File;
        use std::io::BufWriter;
        use std::path::Path;

        let png_path = Path::new("../graft/assets/default_icon.png");
        let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");
        let ico_path = Path::new(&out_dir).join("AppIcon.ico");

        // Convert PNG to ICO with multiple sizes
        let img = image::open(png_path).expect("Failed to load icon PNG");

        let mut icon_dir = ico::IconDir::new(ico::ResourceType::Icon);
        for size in [256, 128, 64, 48, 32, 16] {
            let resized = img.resize_exact(size, size, image::imageops::FilterType::Lanczos3);
            let rgba = resized.to_rgba8();
            let icon_image = ico::IconImage::from_rgba_data(size, size, rgba.into_raw());
            icon_dir
                .add_entry(ico::IconDirEntry::encode(&icon_image).expect("Failed to encode icon"));
        }

        let file = File::create(&ico_path).expect("Failed to create ICO file");
        icon_dir
            .write(BufWriter::new(file))
            .expect("Failed to write ICO");

        // Embed icon using winres
        let mut res = winres::WindowsResource::new();
        res.set_icon(ico_path.to_str().unwrap());
        res.compile().expect("Failed to compile Windows resources");
    }
}
