fn tas_tick(game: &mut crate::game::Game<'_>) {
    if !game.native_playing() {
        return;
    }

    use crate::game::tas::{
        TasMode, TickFrame,
        FLAG_FORWARD, FLAG_BACK, FLAG_LEFT, FLAG_RIGHT,
        FLAG_JUMP, FLAG_SNEAK, FLAG_SPRINT, FLAG_ATTACK, FLAG_USE_ITEM,
    };

    let mut frame_to_apply: Option<TickFrame> = None;
    let mut release_all = false;
    let mut chat_msg: Option<String> = None;

    crate::game::tas::with_tas(|tas| {
        if tas.slowmo_active && tas.mode != TasMode::Idle {
            tas.slowmo_counter = (tas.slowmo_counter + 1) % tas.slowmo_divider.max(1);
            if tas.slowmo_counter != 0 {
                return;
            }
        }

        if tas.mode == TasMode::PausedStep {
            if !tas.step_requested {
                return;
            }
            tas.step_requested = false;
        }

        match tas.mode {
            TasMode::Recording => {
                let frame = TickFrame {
                    mask: tas.live_mask,
                    yaw: tas.live_yaw,
                    pitch: tas.live_pitch,
                    hotbar_slot: tas.active_slot,
                };
                if tas.cursor < tas.tape.frames.len() {
                    tas.tape.frames[tas.cursor] = frame;
                } else {
                    tas.tape.frames.push(frame);
                }
                tas.cursor += 1;
            }
            TasMode::Playing => {
                if let Some(frame) = tas.tape.frames.get(tas.cursor).copied() {
                    frame_to_apply = Some(frame);
                    tas.cursor += 1;
                } else {
                    release_all = true;
                    tas.mode = TasMode::Idle;
                    chat_msg = Some(format!(
                        "[TAS] Воспроизведение макроса '{}' завершено ({} тиков).",
                        tas.tape.name,
                        tas.tape.frames.len()
                    ));
                }
            }
            TasMode::PausedStep | TasMode::Idle => {}
        }
    });

    if let Some(frame) = frame_to_apply {
        crate::game::tas::set_injecting(true);
        game.native_key("key.keyboard.w", frame.has(FLAG_FORWARD));
        game.native_key("key.keyboard.s", frame.has(FLAG_BACK));
        game.native_key("key.keyboard.a", frame.has(FLAG_LEFT));
        game.native_key("key.keyboard.d", frame.has(FLAG_RIGHT));
        game.native_key("key.keyboard.space", frame.has(FLAG_JUMP));
        game.native_key("key.keyboard.left.shift", frame.has(FLAG_SNEAK));
        game.native_key("key.keyboard.left.control", frame.has(FLAG_SPRINT));
        game.native_key("key.mouse.left", frame.has(FLAG_ATTACK));
        game.native_key("key.mouse.right", frame.has(FLAG_USE_ITEM));
        crate::game::tas::set_injecting(false);
    }

    if release_all {
        crate::game::tas::set_injecting(true);
        for k in [
            "key.keyboard.w", "key.keyboard.s", "key.keyboard.a", "key.keyboard.d",
            "key.keyboard.space", "key.keyboard.left.shift", "key.keyboard.left.control",
            "key.mouse.left", "key.mouse.right",
        ] {
            game.native_key(k, false);
        }
        crate::game::tas::set_injecting(false);
    }

    if let Some(msg) = chat_msg {
        game.native_chat(&msg);
    }
}
