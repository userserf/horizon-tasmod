fn tas_command(game: &mut crate::game::Game<'_>, command: &str, ran: &mut bool) {
    let trimmed = command.trim();
    if trimmed == "tas" || trimmed.starts_with("tas ") {
        *ran = true;
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        let sub = parts.get(1).copied().unwrap_or("help");
        let arg = parts.get(2).copied().unwrap_or("default");

        let chat_msg = crate::game::tas::with_tas(|tas| {
            use crate::game::tas::{MacroTape, TasMode};

            match sub {
                "record" | "rec" => {
                    tas.tape = MacroTape::new(arg);
                    tas.cursor = 0;
                    tas.mode = TasMode::Recording;
                    format!("[TAS] Запись макроса '{}' начата!", arg)
                }
                "stop" => {
                    tas.mode = TasMode::Idle;
                    format!(
                        "[TAS] Остановлено на тике {} (всего {} тиков).",
                        tas.cursor,
                        tas.tape.frames.len()
                    )
                }
                "play" => {
                    if !parts.get(2).map_or(true, |s| s.is_empty()) && tas.tape.name != arg {
                        if let Ok(loaded) = MacroTape::load_from_disk(arg) {
                            tas.tape = loaded;
                        }
                    }
                    if tas.tape.frames.is_empty() {
                        String::from("[TAS] Макрос пуст! Сначала запишите его через /tas record <имя>.")
                    } else {
                        tas.cursor = 0;
                        tas.mode = TasMode::Playing;
                        format!(
                            "[TAS] ▶ Воспроизведение макроса '{}' ({} тиков).",
                            tas.tape.name,
                            tas.tape.frames.len()
                        )
                    }
                }
                "save" => {
                    tas.tape.name = arg.to_string();
                    match tas.tape.save_to_disk() {
                        Ok(path) => format!(
                            "[TAS] Сохранено в {} ({} тиков).",
                            path.display(),
                            tas.tape.frames.len()
                        ),
                        Err(err) => format!("[TAS] Ошибка сохранения: {err}"),
                    }
                }
                "load" => match MacroTape::load_from_disk(arg) {
                    Ok(loaded) => {
                        let len = loaded.frames.len();
                        tas.tape = loaded;
                        tas.cursor = 0;
                        tas.mode = TasMode::Idle;
                        format!(
                            "[TAS] Макрос '{arg}' загружен ({len} тиков). Введите /tas play или нажмите P!"
                        )
                    }
                    Err(err) => format!("[TAS] Ошибка чтения 'tas_macros/{arg}.htas': {err}"),
                },
                "clear" => {
                    tas.tape.frames.clear();
                    tas.savestates.clear();
                    tas.cursor = 0;
                    tas.mode = TasMode::Idle;
                    String::from("[TAS] Буфер макроса очищен.")
                }
                _ => String::from(
                    "[TAS] Команды: /tas record <имя>, /tas stop, /tas play <имя>, /tas save <имя>, /tas load <имя>"
                ),
            }
        });

        game.native_chat(&chat_msg);
    }
}
