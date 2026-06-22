// ========================================================================
//  Все константы игры в одном месте — никаких магических чисел
// ========================================================================

// === Мировые константы ===
pub const WORLD_OFFSET_X: f32 = -15.0;
pub const WORLD_OFFSET_Y: f32 = 11.0;

// === Игровое поле (13x9) ===
pub const GRID_COLS: i32 = 13;
pub const GRID_ROWS: i32 = 9;
pub const GRID_MIN_X: f32 = -(GRID_COLS / 2) as f32;
pub const GRID_MAX_X: f32 = (GRID_COLS / 2) as f32;
pub const GRID_MIN_Y: f32 = -(GRID_ROWS / 2) as f32;
pub const GRID_MAX_Y: f32 = (GRID_ROWS / 2) as f32;
pub const TILE_HALF: f32 = 0.5;

// === Слои (z) ===
pub const Z_MAP: f32 = 0.0;
pub const Z_CARPET: f32 = 1.0;
pub const Z_DECOR: f32 = 1.5;
pub const Z_NPC: f32 = 1.8;
pub const Z_CURSOR: f32 = 2.0;
pub const Z_UI: f32 = 3.0;

// === Шейдер ===
pub const SHADER_SCALE: f32 = 0.223;

// === Зум ===
pub const ZOOM_MIN: f32 = 0.4;
pub const ZOOM_MAX: f32 = 1.0;
pub const ZOOM_STEP: f32 = 0.1;

// === Курсор ===
pub const CURSOR_MOVE_DELAY_MS: u64 = 150;
pub const EPSILON: f32 = 0.01;

// === Спрайты/анимация ===
pub const TEXEL_EPSILON: f32 = 0.001;

// === Слоты ===
pub const SLOT_COUNT: usize = 5;

// === Слот-бар ===
pub const SLOT_BAR_Y: f32 = -4.0;
pub const SLOT_BAR_X: f32 = -5.0;
pub const ICON_MODE_X: f32 = 5.0;

// === Текст (цвета, размеры) ===
pub const WHITE: [u8; 3] = [255, 255, 255];
pub const GRAY: [u8; 3] = [200, 200, 200];
pub const GREEN: [u8; 3] = [0, 255, 0];
pub const BTN_TEXT_COLOR: [u8; 3] = [220, 220, 220];

// === Инвентарь ===
pub const INVENTORY_BASE_Y: f32 = SLOT_BAR_Y + 1.0;
pub const INVENTORY_TOP_Y: f32 = INVENTORY_BASE_Y + (INVENTORY_ROWS - 1) as f32;
pub const INVENTORY_ROWS: i32 = 5;
pub const INVENTORY_COLS: i32 = 5;
pub const INV_NONE: i32 = 20;
pub const TAB_TEX: [&str; 2] = ["tex/ui/icon_slots/box.png", "tex/ui/icon_slots/carpet.png"];
pub const INV_REGULAR: &[&str] = &["box", "sign", "rack", "table", "cassa", "ice_cream"];
pub const INV_CARPETS: &[&str] = 
&[
    "carpet", "red_carpet", "green_carpet", "white_carpet", "black_carpet", 
    "iron_panel", "gold_panel", "diamond_panel"
];
pub const INV_TAB_Y: f32 = 2.0;

// === Текстуры курсора по режимам ===
pub const CURSOR_TEX: [&str; 3] = [
    "tex/cursor/def_cursor.png",  // mode 0
    "tex/cursor/cursor.png",      // mode 1 (build)
    "tex/cursor/del_cursor.png",  // mode 2 (delete)
];
pub const CURSOR_ERR_TEX: &str = "tex/cursor/err cursor.png";

// === Текстуры иконки режима ===
pub const MODE_ICON_TEX: [&str; 3] = [
    "tex/ui/mode/standart_mode.png",
    "tex/ui/mode/build_mode.png",
    "tex/ui/mode/del_mode.png",
];

// === Пути к текстурам ковров ===
pub const CARPET_NAMES: [&str; 5] = ["carpet", "red_carpet", "green_carpet", "white_carpet", "black_carpet"];
