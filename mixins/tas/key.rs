fn tas_key(game: &mut crate::game::Game<'_>, key: &str, pressed: bool) {
    if crate::game::tas::is_injecting() {
        return;
    }

    let is_playing_world = game.native_playing();
    let mut chat_msg: Option<String> = None;

    crate::game::tas::with_tas(|tas| {
        use crate::game::tas::{
            TasMode, SavestateEntry,
            KEY_RECORD, KEY_PLAY, KEY_STEP, KEY_SAVESTATE, KEY_LOADSTATE, KEY_SLOWMO,
        };

        if tas.mode != TasMode::Playing {
            tas.on_key_event(key, pressed);
        }

        if !pressed || !is_playing_world {
            return;
        }

        match key {
            KEY_RECORD => {
                if tas.mode == TasMode::Recording {
                    tas.mode = TasMode::Idle;
                    let saved_info = match tas.tape.save_to_disk() {
                        Ok(p) => format!("сохранён в {}", p.display()),
                        Err(_) => String::from("в буфере памяти"),
                    };
                    chat_msg = Some(format!(
                        "[TAS] Запись остановлена: {} тиков ({saved_info}). Нажмите P для воспроизведения!",
                        tas.tape.frames.len()
                    ));
                } else {
                    tas.tape.frames.clear();
                    tas.cursor = 0;
                    tas.mode = TasMode::Recording;
                    chat_msg = Some(String::from("[TAS] ● ЗАПИСЬ МАКРОСА НАЧАТА! Двигайтесь в игре."));
                }
            }
            KEY_PLAY => {
                if tas.mode == TasMode::Playing {
                    tas.mode = TasMode::Idle;
                    chat_msg = Some(String::from("[TAS] Воспроизведение остановлено."));
                } else if tas.tape.frames.is_empty() {
                    chat_msg = Some(String::from("[TAS] Макрос пуст! Сначала запишите его клавишей R или /tas record."));
                } else {
                    tas.cursor = 0;
                    tas.mode = TasMode::Playing;
                    chat_msg = Some(format!(
                        "[TAS] ▶ Воспроизведение макроса '{}' ({} тиков)...",
                        tas.tape.name,
                        tas.tape.frames.len()
                    ));
                }
            }
            KEY_STEP => {
                tas.mode = TasMode::PausedStep;
                tas.step_requested = true;
                chat_msg = Some(format!("[TAS] Шаг +1 тик -> #{}", tas.cursor + 1));
            }
            KEY_SAVESTATE => {
                tas.savestates.push(SavestateEntry {
                    tick_index: tas.cursor,
                    mask: tas.live_mask,
                    yaw: tas.live_yaw,
                    pitch: tas.live_pitch,
                });
                chat_msg = Some(format!(
                    "[TAS] Savestate #{} сохранён на тике {}",
                    tas.savestates.len(),
                    tas.cursor
                ));
            }
            KEY_LOADSTATE => {
                if let Some(saved) = tas.savestates.last().cloned() {
                    tas.cursor = saved.tick_index;
                    if tas.mode == TasMode::Recording {
                        tas.tape.frames.truncate(saved.tick_index);
                    }
                    chat_msg = Some(format!(
                        "[TAS] Откат к Savestate на тике #{}!",
                        saved.tick_index
                    ));
                } else {
                    chat_msg = Some(String::from("[TAS] Нет сохранённых точек (Savestates)."));
                }
            }
            KEY_SLOWMO => {
                tas.slowmo_active = !tas.slowmo_active;
                chat_msg = Some(format!(
                    "[TAS] Slow-Motion: {}",
                    if tas.slowmo_active { "ВКЛЮЧЁН" } else { "ВЫКЛЮЧЕН (20 TPS)" }
                ));
            }
            _ => {}
        }
    });

    if let Some(msg) = chat_msg {
        game.native_chat(&msg);
    }
}
