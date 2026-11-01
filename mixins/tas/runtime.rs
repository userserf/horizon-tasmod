//! Нативный мод `tas` для установки через лаунчер Horizon («ADD A NATIVE MOD FROM GITHUB»).
//!
//! Когда вы вставляете ссылку на GitHub-репозиторий с этим кодом в лаунчер Horizon,
//! лаунчер сам проверяет `compatibility.json` и собирает мод в ваш профиль игры.
//!
//! Управление в игре:
//! - `key.keyboard.r` : Начать / остановить потиковую запись макроса (20 TPS)
//! - `key.keyboard.p` : Воспроизвести записанный макрос в игре
//! - `key.keyboard.f8` : Пошаговый режим (+1 тик)
//! - `key.keyboard.f9` : Создать Savestate (точку сохранения)
//! - `key.keyboard.f10` : Откатиться к Savestate и продолжить запись
//! - Команды в чате: `/tas record <имя>`, `/tas stop`, `/tas play <имя>`, `/tas save <имя>`, `/tas load <имя>`

use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

pub const KEY_RECORD: &str = "key.keyboard.r";
pub const KEY_PLAY: &str = "key.keyboard.p";
pub const KEY_STEP: &str = "key.keyboard.f8";
pub const KEY_SAVESTATE: &str = "key.keyboard.f9";
pub const KEY_LOADSTATE: &str = "key.keyboard.f10";
pub const KEY_SLOWMO: &str = "key.keyboard.g";

pub const FLAG_FORWARD: u16  = 1 << 0; // key.keyboard.w
pub const FLAG_BACK: u16     = 1 << 1; // key.keyboard.s
pub const FLAG_LEFT: u16     = 1 << 2; // key.keyboard.a
pub const FLAG_RIGHT: u16    = 1 << 3; // key.keyboard.d
pub const FLAG_JUMP: u16     = 1 << 4; // key.keyboard.space
pub const FLAG_SNEAK: u16    = 1 << 5; // key.keyboard.left.shift
pub const FLAG_SPRINT: u16   = 1 << 6; // key.keyboard.left.control
pub const FLAG_ATTACK: u16   = 1 << 7; // key.mouse.left
pub const FLAG_USE_ITEM: u16 = 1 << 8; // key.mouse.right

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TickFrame {
    pub mask: u16,
    pub yaw: f32,
    pub pitch: f32,
    pub hotbar_slot: u8,
}

impl TickFrame {
    #[inline]
    pub fn has(&self, flag: u16) -> bool {
        (self.mask & flag) != 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TasMode {
    Idle,
    Recording,
    Playing,
    PausedStep,
}

#[derive(Debug, Clone)]
pub struct SavestateEntry {
    pub tick_index: usize,
    pub mask: u16,
    pub yaw: f32,
    pub pitch: f32,
}

#[derive(Debug, Clone)]
pub struct MacroTape {
    pub name: String,
    pub frames: Vec<TickFrame>,
}

impl MacroTape {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            frames: Vec::with_capacity(2048),
        }
    }

    pub fn save_to_disk(&self) -> std::io::Result<PathBuf> {
        let dir = PathBuf::from("tas_macros");
        fs::create_dir_all(&dir)?;
        let safe_name: String = self
            .name
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
            .collect();
        let path = dir.join(format!("{safe_name}.htas"));
        let file = File::create(&path)?;
        let mut w = BufWriter::new(file);

        writeln!(w, "# Horizon TAS Macro (.htas v1)")?;
        writeln!(w, "# Name: {}", self.name)?;
        for (idx, f) in self.frames.iter().enumerate() {
            writeln!(
                w,
                "{}|0x{:03x}|{:.3}|{:.3}|{}",
                idx + 1,
                f.mask,
                f.yaw,
                f.pitch,
                f.hotbar_slot
            )?;
        }
        w.flush()?;
        Ok(path)
    }

    pub fn load_from_disk(name: &str) -> std::io::Result<Self> {
        let clean = name.trim_end_matches(".htas");
        let path = PathBuf::from("tas_macros").join(format!("{clean}.htas"));
        let file = File::open(&path)?;
        let reader = BufReader::new(file);

        let mut tape = MacroTape::new(clean);
        for line in reader.lines() {
            let line = line?;
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let comment_stripped = trimmed.split('#').next().unwrap_or(trimmed).trim();
            let cols: Vec<&str> = comment_stripped.split('|').collect();
            if cols.len() >= 5 {
                let hex_str = cols[1].trim().trim_start_matches("0x");
                let mask = u16::from_str_radix(hex_str, 16).unwrap_or(0);
                let yaw = cols[2].trim().parse::<f32>().unwrap_or(0.0);
                let pitch = cols[3].trim().parse::<f32>().unwrap_or(0.0);
                let slot = cols[4].trim().parse::<u8>().unwrap_or(1);
                tape.frames.push(TickFrame {
                    mask,
                    yaw,
                    pitch,
                    hotbar_slot: slot,
                });
            }
        }
        Ok(tape)
    }
}

pub struct TasRuntime {
    pub mode: TasMode,
    pub cursor: usize,
    pub live_mask: u16,
    pub live_yaw: f32,
    pub live_pitch: f32,
    pub active_slot: u8,
    pub step_requested: bool,
    pub slowmo_active: bool,
    pub slowmo_divider: u32,
    pub slowmo_counter: u32,
    pub tape: MacroTape,
    pub savestates: Vec<SavestateEntry>,
}

impl Default for TasRuntime {
    fn default() -> Self {
        let mut tape = MacroTape::new("default");
        tape.frames = vec![
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x051, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x051, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x051, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x049, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x049, yaw: -7.20_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x049, yaw: -14.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x049, yaw: -19.80_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x049, yaw: -24.20_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x049, yaw: -27.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x049, yaw: -28.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x045, yaw: -27.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x055, yaw: -24.20_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x045, yaw: -19.80_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x045, yaw: -14.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x045, yaw: -7.20_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x045, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x041, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x020, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x020, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x020, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x020, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x020, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x020, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        TickFrame { mask: 0x020, yaw: 0.00_f32, pitch: 14.00_f32, hotbar_slot: 1 },
        ];
        Self {
            mode: TasMode::Idle,
            cursor: 0,
            live_mask: 0,
            live_yaw: 0.0,
            live_pitch: 0.0,
            active_slot: 1,
            step_requested: false,
            slowmo_active: false,
            slowmo_divider: 4,
            slowmo_counter: 0,
            tape,
            savestates: Vec::new(),
        }
    }
}

static STATE: OnceLock<Mutex<TasRuntime>> = OnceLock::new();

pub fn with_tas<F, R>(f: F) -> R
where
    F: FnOnce(&mut TasRuntime) -> R,
{
    let mtx = STATE.get_or_init(|| Mutex::new(TasRuntime::default()));
    let mut guard = mtx.lock().unwrap_or_else(|e| e.into_inner());
    f(&mut guard)
}

impl TasRuntime {
    pub fn on_key_event(&mut self, key: &str, pressed: bool) {
        let flag = match key {
            "key.keyboard.w" => FLAG_FORWARD,
            "key.keyboard.s" => FLAG_BACK,
            "key.keyboard.a" => FLAG_LEFT,
            "key.keyboard.d" => FLAG_RIGHT,
            "key.keyboard.space" => FLAG_JUMP,
            "key.keyboard.left.shift" | "key.keyboard.right.shift" => FLAG_SNEAK,
            "key.keyboard.left.control" | "key.keyboard.right.control" => FLAG_SPRINT,
            "key.mouse.left" => FLAG_ATTACK,
            "key.mouse.right" => FLAG_USE_ITEM,
            "key.keyboard.1" if pressed => { self.active_slot = 1; 0 }
            "key.keyboard.2" if pressed => { self.active_slot = 2; 0 }
            "key.keyboard.3" if pressed => { self.active_slot = 3; 0 }
            "key.keyboard.4" if pressed => { self.active_slot = 4; 0 }
            "key.keyboard.5" if pressed => { self.active_slot = 5; 0 }
            "key.keyboard.6" if pressed => { self.active_slot = 6; 0 }
            "key.keyboard.7" if pressed => { self.active_slot = 7; 0 }
            "key.keyboard.8" if pressed => { self.active_slot = 8; 0 }
            "key.keyboard.9" if pressed => { self.active_slot = 9; 0 }
            _ => 0,
        };
        if flag != 0 {
            if pressed {
                self.live_mask |= flag;
            } else {
                self.live_mask &= !flag;
            }
        }
    }

    pub fn active_keys_string(&self, mask: u16) -> String {
        let mut s = String::with_capacity(12);
        s.push(if (mask & FLAG_FORWARD) != 0 { 'W' } else { '.' });
        s.push(if (mask & FLAG_LEFT) != 0 { 'A' } else { '.' });
        s.push(if (mask & FLAG_BACK) != 0 { 'S' } else { '.' });
        s.push(if (mask & FLAG_RIGHT) != 0 { 'D' } else { '.' });
        s.push(' ');
        s.push(if (mask & FLAG_JUMP) != 0 { 'J' } else { '.' });
        s.push(if (mask & FLAG_SPRINT) != 0 { 'P' } else { '.' });
        s.push(if (mask & FLAG_SNEAK) != 0 { 'N' } else { '.' });
        s
    }

    pub fn format_hud_status(&self) -> String {
        let mode_label = match self.mode {
            TasMode::Idle => "IDLE",
            TasMode::Recording => "REC",
            TasMode::Playing => "PLAY",
            TasMode::PausedStep => "STEP",
        };
        let shown_mask = match self.mode {
            TasMode::Playing => self
                .tape
                .frames
                .get(self.cursor.saturating_sub(1))
                .map(|f| f.mask)
                .unwrap_or(0),
            _ => self.live_mask,
        };
        format!(
            "[TAS: {}] {} | Tick {}/{} | [{}]",
            mode_label,
            self.tape.name,
            self.cursor,
            self.tape.frames.len(),
            self.active_keys_string(shown_mask)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macro_record_and_keys() {
        with_tas(|tas| {
            tas.tape = MacroTape::new("test_run");
            tas.mode = TasMode::Recording;
            tas.cursor = 0;
            tas.on_key_event("key.keyboard.w", true);
            tas.on_key_event("key.keyboard.space", true);

            tas.tape.frames.push(TickFrame {
                mask: tas.live_mask,
                yaw: 0.0,
                pitch: 0.0,
                hotbar_slot: 1,
            });
            tas.cursor += 1;

            assert_eq!(tas.tape.frames.len(), 1);
            assert!(tas.tape.frames[0].has(FLAG_FORWARD));
            assert!(tas.tape.frames[0].has(FLAG_JUMP));
        });
    }
}
