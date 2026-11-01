// Хук `native_command`: обрабатывает чат-команды /tas record, /tas play, /tas stop, /tas save, /tas load
{
    let trimmed = command.trim();
    if trimmed == "tas" || trimmed.starts_with("tas ") {
        *ran = true;
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        let sub = parts.get(1).copied().unwrap_or("help");
        let arg = parts.get(2).copied().unwrap_or("default");

        crate::game::tas::with_tas(|tas| {
            use crate::game::tas::{MacroTape, TasMode};

            match sub {
                "record" | "rec" => {
                    tas.tape = MacroTape::new(arg);
                    tas.cursor = 0;
                    tas.mode = TasMode::Recording;
                    game.native_chat(&format!("[TAS] Запись макроса '{}' начата!", arg));
                }
                "stop" => {
                    tas.mode = TasMode::Idle;
                    game.native_chat(&format!(
                        "[TAS] Остановлено на тике {} (всего {} тиков).",
                        tas.cursor,
                        tas.tape.frames.len()
                    ));
                }
                "play" => {
                    if !parts.get(2).map_or(true, |s| s.is_empty()) && tas.tape.name != arg {
                        if let Ok(loaded) = MacroTape::load_from_disk(arg) {
                            tas.tape = loaded;
                        }
                    }
                    if tas.tape.frames.is_empty() {
                        game.native_chat("[TAS] Макрос пуст! Сначала запишите его через /tas record <имя>.");
                    } else {
                        tas.cursor = 0;
                        tas.mode = TasMode::Playing;
                        game.native_chat(&format!(
                            "[TAS] ▶ Воспроизведение макроса '{}' ({} тиков).",
                            tas.tape.name,
                            tas.tape.frames.len()
                        ));
                    }
                }
                "save" => {
                    tas.tape.name = arg.to_string();
                    match tas.tape.save_to_disk() {
                        Ok(path) => game.native_chat(&format!(
                            "[TAS] Сохранено в {} ({} тиков).",
                            path.display(),
                            tas.tape.frames.len()
                        )),
                        Err(err) => game.native_chat(&format!("[TAS] Ошибка сохранения: {err}")),
                    }
                }
                "load" => match MacroTape::load_from_disk(arg) {
                    Ok(loaded) => {
                        let len = loaded.frames.len();
                        tas.tape = loaded;
                        tas.cursor = 0;
                        tas.mode = TasMode::Idle;
                        game.native_chat(&format!(
                            "[TAS] Макрос '{arg}' загружен ({len} тиков). Введите /tas play или нажмите P!"
                        ));
                    }
                    Err(err) => game.native_chat(&format!("[TAS] Ошибка чтения 'tas_macros/{arg}.htas': {err}")),
                },
                "clear" => {
                    tas.tape.frames.clear();
                    tas.savestates.clear();
                    tas.cursor = 0;
                    tas.mode = TasMode::Idle;
                    game.native_chat("[TAS] Буфер макроса очищен.");
                }
                _ => {
                    game.native_chat("[TAS] Команды: /tas record <имя>, /tas stop, /tas play <имя>, /tas save <имя>, /tas load <имя>");
                }
            }
        });
    }
}
