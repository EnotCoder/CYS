use specs::Entity;

/// Панель-подложка. Позиция и размер задаются в мировых координатах UI
/// (map_size=1.0), entity — спрайт-сущность подложки в ECS.
pub struct Panel {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub alpha: f32,
    pub entity: Option<Entity>,
}

impl Panel {
    pub fn new(x: f32, y: f32, w: f32, h: f32, alpha: f32) -> Self {
        Self { x, y, w, h, alpha, entity: None }
    }
}

/// Кнопка: прямоугольная подложка (bg) + текстовый спрайт (text) по центру.
pub struct Button {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub label: String,
    pub font_size: f32,
    pub bg: Option<Entity>,
    pub text: Option<Entity>,
}

impl Button {
    pub fn new(x: f32, y: f32, w: f32, h: f32, label: &str) -> Self {
        Self { x, y, w, h, label: label.to_string(), font_size: 48.0, bg: None, text: None }
    }
}

/// Чекбокс: галочка (box_entity) и подпись справа от неё (label_entity).
/// sprite_key-и хранятся, чтобы удалять спрайты из кэша при пересоздании.
pub struct Checkbox {
    pub x: f32,
    pub y: f32,
    pub label: String,
    pub font_size: f32,
    pub checked: bool,
    /// Спрайт-галочка (tex/ui/true.png или tex/ui/false.png)
    pub box_entity: Option<Entity>,
    pub box_sprite_key: Option<u64>,
    /// Текст подписи справа от галочки
    pub label_entity: Option<Entity>,
    pub label_sprite_key: Option<u64>,
}

impl Checkbox {
    pub fn new(x: f32, y: f32, label: &str, checked: bool) -> Self {
        Self { x, y, label: label.to_string(), font_size: 48.0, checked, box_entity: None, box_sprite_key: None, label_entity: None, label_sprite_key: None }
    }
}

/// Горизонтальный слайдер: дорожка (track), ползунок (thumb) и подпись.
/// dragging — флаг активного перетаскивания ползунка мышью.
pub struct Slider {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub min: f32,
    pub max: f32,
    pub value: f32,
    pub label: String,
    pub font_size: f32,
    pub dragging: bool,
    pub track: Option<Entity>,
    pub thumb: Option<Entity>,
    pub label_entity: Option<Entity>,
    pub label_sprite_key: Option<u64>,
}

impl Slider {
    pub fn new(x: f32, y: f32, label: &str, min: f32, max: f32, value: f32) -> Self {
        Self {
            x, y, width: 3.0, height: 0.3,
            min, max, value,
            label: label.to_string(),
            font_size: 50.0,
            dragging: false,
            track: None, thumb: None,
            label_entity: None, label_sprite_key: None,
        }
    }
}
