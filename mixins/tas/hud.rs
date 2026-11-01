fn tas_hud(
    game: &mut crate::game::Game<'_>,
    batch: &mut horizon_client::hud::HudBatch,
    _corners: &mut horizon_client::extras_hud::Corners,
) {
    let (text, color) = crate::game::tas::with_tas(|tas| {
        let c = match tas.mode {
            crate::game::tas::TasMode::Recording => 0xFF5555,
            crate::game::tas::TasMode::Playing => 0x55FF55,
            crate::game::tas::TasMode::PausedStep => 0x55FFFF,
            crate::game::tas::TasMode::Idle => 0xFFFF55,
        };
        (tas.format_hud_status(), c)
    });

    let w = game.native_text_width(&text) + 4.0;
    let (x, y) = (6.0_f32, 6.0_f32);
    game.native_fill(batch, [x, y, w, 12.0], 0x7800_0000);
    game.native_text(batch, &text, [x + 2.0, y + 2.0], color, true);
}
