// Хук `native_hud`: отрисовывает панель статуса TAS на экране игры
{
    crate::game::tas::with_tas(|tas| {
        let text = tas.format_hud_status();
        let width = (text.len() as f32) * 6.0 + 12.0;
        let height = 18.0;
        let (x, y) = corners.place(crate::extras::Corner::TopLeft, width, height);

        crate::game::Game::native_fill(batch, x, y, width, height, [10, 14, 23, 205]);
        let color = match tas.mode {
            crate::game::tas::TasMode::Recording => [251, 113, 133, 255],
            crate::game::tas::TasMode::Playing => [52, 211, 153, 255],
            crate::game::tas::TasMode::PausedStep => [56, 189, 248, 255],
            crate::game::tas::TasMode::Idle => [226, 232, 240, 230],
        };
        crate::game::Game::native_text(batch, x + 6.0, y + 5.0, &text, color);
    });
}
