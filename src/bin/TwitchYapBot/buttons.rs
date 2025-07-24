//! Audio playback utilities and button image logic for TwitchYapBot
//!
//! This module provides sound arrays, audio playback, and button image loading for the TwitchYapBot GUI.

use rand::Rng;
use std::io::Cursor;
use eframe::egui;
use image::imageops::FilterType;

// Include generated sound arrays for DeathScream and Angelic
include!(concat!(env!("OUT_DIR"), "/death_screams_generated.rs"));
include!(concat!(env!("OUT_DIR"), "/angelic_sounds_generated.rs"));

/// Play a random sound from the provided slice in a background thread.
pub fn play_random_sound(sounds: &[&[u8]]) {
    let idx = rand::thread_rng().gen_range(0..sounds.len());
    let sound = sounds[idx].to_vec();
    std::thread::spawn(move || {
        if let Ok((_stream, stream_handle)) = rodio::OutputStream::try_default() {
            if let Ok(sink) = rodio::Sink::try_new(&stream_handle) {
                let cursor = Cursor::new(sound);
                if let Ok(source) = rodio::Decoder::new(cursor) {
                    sink.append(source);
                    sink.sleep_until_end();
                }
            }
        }
    });
}

/// Loads a button image as a texture and returns the TextureHandle.
pub fn load_button_texture(ctx: &egui::Context, name: &str, image_bytes: &'static [u8]) -> egui::TextureHandle {
    let image = image::load_from_memory(image_bytes).unwrap().to_rgba8();
    let (width, height) = image.dimensions();
    let size = [width as usize, height as usize];
    ctx.load_texture(
        name,
        egui::ColorImage::from_rgba_unmultiplied(size, &image.into_raw()),
        egui::TextureOptions::LINEAR,
    )
}

/// Draws the settings cog button and returns true if clicked.
pub fn settings_cog_button(ctx: &egui::Context, size: f32) -> egui::ImageButton {
    let cog_image = include_bytes!("../../../resources/buttons/SettingsCog.png");
    let cache_id = format!("settings_cog_{}", size);
    let id = egui::Id::new(cache_id);
    let texture = ctx.data(|data| {
        data.get_temp::<egui::TextureHandle>(id)
    });
    let cog_texture = if let Some(tex) = texture {
        tex
    } else {
        // Decode and resize the image only once per size
        let image = image::load_from_memory(cog_image).unwrap().to_rgba8();
        let (orig_w, orig_h) = image.dimensions();
        let target_size = size.round() as u32;
        let resized = if orig_w != target_size || orig_h != target_size {
            image::imageops::resize(&image, target_size, target_size, FilterType::Lanczos3)
        } else {
            image
        };
        let size_arr = [resized.width() as usize, resized.height() as usize];
        let tex = ctx.load_texture(
            &format!("settings_cog_{}", size),
            egui::ColorImage::from_rgba_unmultiplied(size_arr, &resized.into_raw()),
            egui::TextureOptions::LINEAR,
        );
        ctx.data_mut(|data| {
            data.insert_temp(id, tex.clone());
        });
        tex
    };
    egui::ImageButton::new((cog_texture.id(), egui::vec2(size, size)))
}

/// Draws the revive button and returns true if clicked.
pub fn revive_button(ctx: &egui::Context) -> egui::ImageButton {
    let revive_image = include_bytes!("../../../resources/buttons/revive.png");
    let revive_texture = load_button_texture(ctx, "revive_button", revive_image);
    egui::ImageButton::new((revive_texture.id(), egui::vec2(121.0, 45.0)))
}

/// Draws the murder button and returns true if clicked.
pub fn murder_button(ctx: &egui::Context) -> egui::ImageButton {
    let murder_image = include_bytes!("../../../resources/buttons/murder.png");
    let murder_texture = load_button_texture(ctx, "murder_button", murder_image);
    egui::ImageButton::new((murder_texture.id(), egui::vec2(121.0, 45.0)))
}

/// Draws the yap button and returns true if clicked.
pub fn yap_button(ctx: &egui::Context) -> egui::ImageButton {
    let yap_image = include_bytes!("../../../resources/buttons/yap.png");
    let yap_texture = load_button_texture(ctx, "yap_button", yap_image);
    egui::ImageButton::new((yap_texture.id(), egui::vec2(121.0, 45.0)))
}
