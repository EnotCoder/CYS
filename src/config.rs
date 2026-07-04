use std::sync::OnceLock;

fn env_flag(name: &str) -> bool {
    std::env::var(name).ok().as_deref() == Some("1")
}

pub struct DebugFlags {
    pub show_fps: bool,
    pub show_bounds: bool,
    pub show_pathfinding: bool,
}

static DEBUG_FLAGS: OnceLock<DebugFlags> = OnceLock::new();

pub fn debug_flags() -> &'static DebugFlags {
    DEBUG_FLAGS.get_or_init(|| {
        DebugFlags {
            show_fps: env_flag("SHOW_FPS"),
            show_bounds: env_flag("SHOW_BOUNDS"),
            show_pathfinding: env_flag("SHOW_PATHFINDING"),
        }
    })
}

pub fn init() {
    dotenvy::dotenv().ok();
    env_logger::init();
}
