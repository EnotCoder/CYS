// ========================================================================
//  Все константы игры в одном месте — никаких магических чисел
// ========================================================================

// === Мировые константы ===
pub const WORLD_OFFSET_X: f32 = -15.0;
pub const WORLD_OFFSET_Y: f32 = 11.0;

// === Игровое поле (grid 9x9) ===
pub const GRID_MIN: f32 = -4.0;
pub const GRID_MAX: f32 = 4.0;

// === Слои (z) ===
pub const Z_MAP: f32 = 0.0;
pub const Z_CARPET: f32 = 1.0;
pub const Z_DECOR: f32 = 1.5;
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

// === Слоты ===
pub const SLOT_COUNT: usize = 5;

// === Инвентарь ===
pub const INVENTORY_ROWS: i32 = 5;
pub const INVENTORY_COLS: i32 = 5;
pub const INV_ITEMS: &[&str] = &["box", "sign", "rack", "table", "cassa", 
"carpet", "red_carpet", "green_carpet", "white_carpet", "black_carpet"];

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

// === Слот-бар ===
pub const SLOT_BAR_Y: f32 = -4.0;
