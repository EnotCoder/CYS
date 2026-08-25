// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  Все константы игры в одном месте — никаких магических чисел
// ========================================================================

// === Мировые константы ===
pub const WORLD_OFFSET_X: f32 = -21.0;
pub const WORLD_OFFSET_Y: f32 = 14.0;

pub const TILE_HALF: f32 = 0.5;

// === Слои (z) ===
pub const Z_MAP: f32 = 0.0;
pub const Z_CARPET: f32 = 1.0;
pub const Z_LIGHT: f32 = 1.2;
pub const Z_DECOR: f32 = 1.5;
pub const Z_NPC: f32 = 1.8;
pub const Z_CURSOR: f32 = 2.0;
pub const Z_UI: f32 = 3.0;

// === Шейдер ===
pub const SHADER_SCALE: f32 = 0.223;

// === Зум ===
pub const ZOOM_MIN: f32 = 0.4;
pub const ZOOM_MAX: f32 = 0.8;

// === Камера ===
pub const CAMERA_SPEED: f32 = 5.0;
pub const CAMERA_MAP_MIN_X: f32 = -20.0;
pub const CAMERA_MAP_MAX_X: f32 = 18.0;
pub const CAMERA_MAP_MIN_Y: f32 = -13.0;
pub const CAMERA_MAP_MAX_Y: f32 = 13.0;

// === NPC ===
/// Масштаб спрайта покупателя (движение/анимация — в scripts/config.lua)
pub const NPC_SCALE: f32 = 1.5;

// === Текстовые текстуры игрока ===
pub const TEX_PLAYER_IDLE: &str = "tex/characters/player/player.png";
pub const TEX_PLAYER_WALK_1: &str = "tex/characters/player/player_walk_1.png";
pub const TEX_PLAYER_WALK_2: &str = "tex/characters/player/player_walk_2.png";

pub const TEX_BOB_IDLE: &str = "tex/characters/bob/bob.png";
pub const TEX_BOB_WALK_1: &str = "tex/characters/bob/bob_walk_1.png";
pub const TEX_BOB_WALK_2: &str = "tex/characters/bob/bob_walk_2.png";

pub const TEX_SASHA_IDLE: &str = "tex/characters/sasha/sasha.png";
pub const TEX_SASHA_WALK_1: &str = "tex/characters/sasha/sasha_walk_1.png";
pub const TEX_SASHA_WALK_2: &str = "tex/characters/sasha/sasha_walk_2.png";

// === Shopping NPC ===
// (тайминги и экономика покупателя — в scripts/config.lua)

// === Курсор ===
pub const CURSOR_MOVE_DELAY_MS: u64 = 150;
pub const EPSILON: f32 = 0.01;
// MouseButton теперь из winit::mouse

// === Окно ===
pub const WINDOW_WIDTH: u32 = 1200;
pub const WINDOW_HEIGHT: u32 = 800;

// === Спрайты/анимация ===
pub const TEXEL_EPSILON: f32 = 0.001;

// === Слоты ===
pub const SLOT_COUNT: usize = 5;

// === Слот-бар ===
pub const SLOT_BAR_Y: f32 = -4.0;
pub const SLOT_BAR_X: f32 = -5.0;
pub const ICON_MODE_X: f32 = 5.0;
pub const INV_BTN_X: f32 = SLOT_BAR_X + SLOT_COUNT as f32;
pub const TEX_INV_BUTTON: &str = "tex/ui/icon_slots/inv.png";

// === Текст (цвета, размеры) ===
pub const WHITE: [u8; 3] = [255, 255, 255];
pub const RED: [u8; 3] = [255, 60, 60];
pub const GRAY: [u8; 3] = [200, 200, 200];
pub const GREEN: [u8; 3] = [0, 255, 0];
pub const BTN_TEXT_COLOR: [u8; 3] = [220, 220, 220];

// === UI-анимации ===
// Длительность пульса счётчиков/рамок (сек) и скорость фейда подсказок.
pub const UI_PULSE_SECS: f64 = 0.3;
pub const UI_FADE_SPEED: f32 = 6.0;

// === Общие пути ===
pub const TEX_UI_ICON_SLOTS_OBJECT_DIR: &str = "tex/ui/icon_slots/icon_slots_object/";
pub const TEX_UI_ICON_SLOTS_MAP_DIR: &str = "tex/ui/icon_slots/";
pub const TEX_FALLBACK: &str = "tex/dev_tools/null.png";

pub const MAP_FILE: &str = "map.txt";
pub const BASEMENT_FILE: &str = "basement.txt";

// === Инвентарь ===
pub const INVENTORY_BASE_Y: f32 = SLOT_BAR_Y + 1.0;
pub const INVENTORY_ROWS: i32 = 5;
pub const INVENTORY_COLS: i32 = 5;
pub const INV_NONE: i32 = INVENTORY_ROWS * (INVENTORY_COLS - 1);
pub const TAB_TEX: [&str; 5] = ["tex/ui/icon_slots/icon_slots_object/regular/box.png", "tex/ui/icon_slots/icon_slots_object/carpets/blue_carpet.png", "tex/ui/icon_slots/icon_slots_object/walldecor/welcome.png", "tex/ui/icon_slots/icon_slots_object/outdoor/tree.png", "tex/ui/icon_slots/icon_slots_object/light/lamp.png"];
pub const SLOT_CURSOR_TEX: &str = "tex/ui/icon_slots/cursor.png";
pub const INV_REGULAR: &[&str] = &["box", "sign", "rack", "table", "cassa", "ice_cream", "arcade_machine", "candies", "fence", "basement"];
pub const INV_LIGHT: &[&str] = &["lamp", "big_lamp"];
pub const INV_CARPETS: &[&str] = 
&[
    "blue_carpet", "red_carpet", "green_carpet", "white_carpet", "black_carpet", 
    "iron_panel", "gold_panel", "diamond_panel"
];
pub const INV_WALLDECOR: &[&str] = &["welcome", "fnaf", "watch"];
pub const INV_OUTDOOR: &[&str] = &["street_fence", "tree", "pink_flower", "blue_flower", "yellow_flower", "red_flower", "white_flower", "street_ice_cream", "trashcan", "bench"];
pub const OUTDOOR_NAMES: [&str; 10] = ["street_fence", "tree", "pink_flower", "blue_flower", "yellow_flower", "red_flower", "white_flower", "street_ice_cream", "trashcan", "bench"];
pub const FLOWER_NAMES: [&str; 5] = ["pink_flower", "blue_flower", "yellow_flower", "red_flower", "white_flower"];
pub const INV_TAB_Y: f32 = 2.0;

// === Текстуры курсора по режимам ===
pub const CURSOR_TEX: [&str; 3] = [
    "tex/ui/cursor/def_cursor.png",  // mode 0
    "tex/ui/cursor/cursor.png",      // mode 1 (build)
    "tex/ui/cursor/del_cursor.png",  // mode 2 (delete)
];
pub const CURSOR_ERR_TEX: &str = "tex/ui/cursor/err cursor.png";

// === Active toggle ===
/// Позиция спрайта вкл/выкл NPC (левее mode)
pub const ACTIVE_X: f32 = ICON_MODE_X + 1.0;
pub const TEX_ACTIVE: &str = "tex/ui/active/active.png";
pub const TEX_NO_ACTIVE: &str = "tex/ui/active/no_active.png";

// === Лого в правом нижнем углу ===
pub const LOGO_UI_X: f32 = ACTIVE_X + 0.2;
pub const LOGO_UI_Y: f32 = SLOT_BAR_Y + 0.1;
pub const TEX_MY_LOGO: &str = "tex/ui/my_logo.png";

// === Текстуры иконки режима ===
pub const MODE_ICON_TEX: [&str; 3] = [
    "tex/ui/mode/standart_mode.png",
    "tex/ui/mode/build_mode.png",
    "tex/ui/mode/del_mode.png",
];

// === Пути к текстурам ковров ===
pub const CARPET_NAMES: [&str; 8] = ["blue_carpet", "red_carpet", "green_carpet", "white_carpet", "black_carpet", "iron_panel", "gold_panel", "diamond_panel"];

// === Меню ===
pub const BTN_X: f32 = 0.0;
pub const BTN_Y: f32 = -0.5;
pub const BTN_W: f32 = 2.0;
pub const BTN_H: f32 = 0.8;
pub const QUIT_X: f32 = 0.0;
pub const QUIT_Y: f32 = -1.5;
pub const QUIT_W: f32 = 2.0;
pub const QUIT_H: f32 = 0.8;
pub const MENU_MAP_SIZE: f32 = 0.8;
pub const LOGO_X: f32 = 0.0;
pub const LOGO_Y: f32 = 2.0;
pub const LOGO_W: f32 = 2.5;
pub const LOGO_H: f32 = 2.5;
pub const FONT_SIZE_BTN: f32 = 48.0;
pub const FONT_SIZE_LOGO: f32 = 128.0;
pub const FONT_SIZE_ALPHA: f32 = 64.0;

// === Рендер ===
pub const DEPTH_CLEAR: f32 = 1.0;
pub const DESIRED_FRAME_LATENCY: u32 = 2;

// === Динамический storage buffer ===
/// Максимум спрайтов, которые могут быть записаны в dynamic storage buffer за кадр.
pub const MAX_DYNAMIC_SPRITES: usize = 8192;

/// Максимальное количество источников света в кадре.
pub const MAX_LIGHTS: usize = 128;

// === Индексный буфер квадрата ===
pub const QUAD_INDICES: [u16; 6] = [0, 1, 2, 2, 3, 0];
