// Хук `native_key`: вызывается при нажатии/отпускании любой клавиши в Horizon
{
    crate::game::tas::with_tas(|tas| {
        use crate::game::tas::{
            TasMode, SavestateEntry,
            KEY_RECORD, KEY_PLAY, KEY_STEP, KEY_SAVESTATE, KEY_LOADSTATE, KEY_SLOWMO,
        };

        // Если макрос сейчас не проигрывается — считываем живой ввод игрока
        if tas.mode != TasMode::Playing {
            tas.on_key_event(key, pressed);
        }

        if !pressed || !game.native_playing() {
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
                    game.native_chat(&format!(
                        "[TAS] Запись остановлена: {} тиков ({saved_info}). Нажмите P для воспроизведения!",
                        tas.tape.frames.len()
                    ));
                } else {
                    tas.tape.frames.clear();
                    tas.cursor = 0;
                    tas.mode = TasMode::Recording;
                    game.native_chat("[TAS] ● ЗАПИСЬ МАКРОСА НАЧАТА! Двигайтесь в игре.");
                }
            }
            KEY_PLAY => {
                if tas.mode == TasMode::Playing {
                    tas.mode = TasMode::Idle;
                    game.native_chat("[TAS] Воспроизведение остановлено.");
                } else if tas.tape.frames.is_empty() {
                    game.native_chat("[TAS] Макрос пуст! Сначала запишите его клавишей R или /tas record.");
                } else {
                    tas.cursor = 0;
                    tas.mode = TasMode::Playing;
                    game.native_chat(&format!(
                        "[TAS] ▶ Воспроизведение макроса '{}' ({} тиков)...",
                        tas.tape.name,
                        tas.tape.frames.len()
                    ));
                }
            }
            KEY_STEP => {
                tas.mode = TasMode::PausedStep;
                tas.step_requested = true;
                game.native_chat(&format!("[TAS] Шаг +1 тик -> #{}", tas.cursor + 1));
            }
            KEY_SAVESTATE => {
                tas.savestates.push(SavestateEntry {
                    tick_index: tas.cursor,
                    mask: tas.live_mask,
                    yaw: tas.live_yaw,
                    pitch: tas.live_pitch,
                });
                game.native_chat(&format!(
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
                    game.native_chat(&format!(
                        "[TAS] Откат к Savestate на тике #{}!",
                        saved.tick_index
                    ));
                } else {
                    game.native_chat("[TAS] Нет сохранённых точек (Savestates).");
                }
            }
            KEY_SLOWMO => {
                tas.slowmo_active = !tas.slowmo_active;
                game.native_chat(&format!(
                    "[TAS] Slow-Motion: {}",
                    if tas.slowmo_active { "ВКЛЮЧЁН" } else { "ВЫКЛЮЧЕН (20 TPS)" }
                ));
            }
            _ => {}
        }
    });
}
