use vulkano::format::Format;
// Generate the winit <-> conrod type conversion fns.
conrod_winit::conversion_fns!();

pub fn format_is_srgb(format: Format) -> bool {
    use vulkano::format::Format::*;
    match format {
        R8Srgb
        | R8G8Srgb
        | R8G8B8Srgb
        | B8G8R8Srgb
        | R8G8B8A8Srgb
        | B8G8R8A8Srgb
        | A8B8G8R8SrgbPack32
        | BC1_RGBSrgbBlock
        | BC1_RGBASrgbBlock
        | BC2SrgbBlock
        | BC3SrgbBlock
        | BC7SrgbBlock
        | ETC2_R8G8B8SrgbBlock
        | ETC2_R8G8B8A1SrgbBlock
        | ETC2_R8G8B8A8SrgbBlock
        | ASTC_4x4SrgbBlock
        | ASTC_5x4SrgbBlock
        | ASTC_5x5SrgbBlock
        | ASTC_6x5SrgbBlock
        | ASTC_6x6SrgbBlock
        | ASTC_8x5SrgbBlock
        | ASTC_8x6SrgbBlock
        | ASTC_8x8SrgbBlock
        | ASTC_10x5SrgbBlock
        | ASTC_10x6SrgbBlock
        | ASTC_10x8SrgbBlock
        | ASTC_10x10SrgbBlock
        | ASTC_12x10SrgbBlock
        | ASTC_12x12SrgbBlock => true,
        _ => false,
    }
}
