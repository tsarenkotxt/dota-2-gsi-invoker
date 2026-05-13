pub fn spell_image(asset_name: &str) -> Option<&'static [u8]> {
    match asset_name {
        "Alacrity.webp" => Some(include_bytes!("../assets/Alacrity.webp")),
        "Chaos_Meteor.webp" => Some(include_bytes!("../assets/Chaos_Meteor.webp")),
        "Cold_Snap.webp" => Some(include_bytes!("../assets/Cold_Snap.webp")),
        "Deafening_Blast.webp" => Some(include_bytes!("../assets/Deafening_Blast.webp")),
        "EMP.webp" => Some(include_bytes!("../assets/EMP.webp")),
        "Forge_Spirit.webp" => Some(include_bytes!("../assets/Forge_Spirit.webp")),
        "Ghost_Walk.webp" => Some(include_bytes!("../assets/Ghost_Walk.webp")),
        "Ice_Wall.webp" => Some(include_bytes!("../assets/Ice_Wall.webp")),
        "Sun_Strike.webp" => Some(include_bytes!("../assets/Sun_Strike.webp")),
        "Tornado.webp" => Some(include_bytes!("../assets/Tornado.webp")),
        _ => None,
    }
}
