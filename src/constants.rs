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
pub const GRID_BOUNDARY_ADJUST: i32 = 1;

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
pub const ZOOM_MAX: f32 = 0.8;
pub const ZOOM_STEP: f32 = 0.1;

// === NPC ===
pub const NPC_SPEED: f32 = 3.0;
pub const NPC_PAUSE_DURATION: f64 = 0.3;
pub const WALK_ANIM_INTERVAL: f64 = 0.3;
pub const NPC_SCALE: f32 = 1.5;

// === Текстовые текстуры игрока ===
pub const TEX_PLAYER_IDLE: &str = "tex/characters/player.png";
pub const TEX_PLAYER_WALK_1: &str = "tex/characters/player_walk_1.png";
pub const TEX_PLAYER_WALK_2: &str = "tex/characters/player_walk_2.png";

// === Курсор ===
pub const CURSOR_MOVE_DELAY_MS: u64 = 150;
pub const EPSILON: f32 = 0.01;
pub const MOUSE_BUTTON_LEFT: usize = 0;

// === Окно ===
pub const WINDOW_WIDTH: u32 = 1000;
pub const WINDOW_HEIGHT: u32 = 800;

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

// === Общие пути ===
pub const TEX_UI_ICON_SLOTS_DIR: &str = "tex/ui/icon_slots/";
pub const TEX_FALLBACK: &str = "tex/null.png";

pub const MAP_FILE: &str = "map.txt";

// === Инвентарь ===
pub const INVENTORY_BASE_Y: f32 = SLOT_BAR_Y + 1.0;
pub const INVENTORY_TOP_Y: f32 = INVENTORY_BASE_Y + (INVENTORY_ROWS - 1) as f32;
pub const INVENTORY_ROWS: i32 = 5;
pub const INVENTORY_COLS: i32 = 5;
pub const INV_NONE: i32 = INVENTORY_ROWS * (INVENTORY_COLS - 1);
pub const TAB_TEX: [&str; 2] = ["tex/ui/icon_slots/box.png", "tex/ui/icon_slots/carpet.png"];
pub const SLOT_CURSOR_TEX: &str = "tex/ui/icon_slots/cursor.png";
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

// === Рендер ===
pub const DEPTH_CLEAR: f32 = 1.0;
pub const DESIRED_FRAME_LATENCY: u32 = 2;

// === Индексный буфер квадрата ===
pub const QUAD_INDICES: [u16; 6] = [0, 1, 2, 2, 3, 0];
