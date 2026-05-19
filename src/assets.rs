use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "target/dist"]
pub struct Assets;
