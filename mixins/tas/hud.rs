// Хук `native_hud`: отрисовывает панель статуса TAS на экране игры
{
    crate::game::tas::with_tas(|tas| {
        let text = tas.format_hud_status();
        let w = game.native_text_width(&text) + 4.0;
        let (x, y) = corners.place(horizon_client::extras_hud::Corner::TopLeft, w, 12.0);

        game.native_fill(batch, [x, y, w, 12.0], 0x7800_0000);
        let color = match tas.mode {
            crate::game::tas::TasMode::Recording => 0xFF5555,
            crate::game::tas::TasMode::Playing => 0x55FF55,
            crate::game::tas::TasMode::PausedStep => 0x55FFFF,
            crate::game::tas::TasMode::Idle => 0xFFFF55,
        };
        game.native_text(batch, &text, [x + 2.0, y + 2.0], color, true);
    });
}
