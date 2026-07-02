use specs::Entity;

pub struct Panel {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub alpha: f32,
    pub entity: Option<Entity>,
}

impl Panel {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h, alpha: 0.7, entity: None }
    }
}

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
