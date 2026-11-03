// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  helpers — общие заготовки для тестов: ECS без GPU, размещение объектов,
//  фейковый источник ввода (InputSource) и перевод UI-координат в пиксели.
// ========================================================================

#![allow(dead_code)]

use specs::WorldExt;
use winit::event::{DeviceEvent, MouseButton, WindowEvent};
use winit::keyboard::KeyCode;

use crate::core::constants::SHADER_SCALE;
use crate::data::{is_carpet_name, is_light_name, make_slot};
use crate::ecs::components::FoodStorage;
use crate::input::platform::InputSource;
use crate::{EcsAdapter, GroupInfoResource, ObjectTag};

/// Пустой ECS-мир (все ресурсы зарегистрированы, карта не загружена).
pub fn new_ecs() -> EcsAdapter {
    EcsAdapter::new()
}

/// ECS-мир с загруженной картой магазина.
pub fn ecs_with_map() -> EcsAdapter {
    let mut ecs = EcsAdapter::new();
    crate::data::map::load_map_to_ecs(&mut ecs);
    ecs
}

/// Ставит объект из каталога напрямую (без проверок денег/доступности)
/// и вешает ObjectTag на все сущности группы, как это делает игра.
pub fn add_object(ecs: &mut EcsAdapter, name: &str, x: i32, y: i32) -> u32 {
    let slot = make_slot(name);
    let gid = ecs.add_group_object(
        x, y,
        slot.obj.width, slot.obj.height,
        name,
        slot.obj.path,
        slot.obj.texture_frame,
        slot.obj.texture_count,
        is_carpet_name(name),
        is_light_name(name),
        slot.obj.animated,
        slot.obj.frame_paths,
    );
    let entities = group_entities(ecs, gid);
    for entity in entities {
        ecs.world
            .write_storage::<ObjectTag>()
            .insert(entity, ObjectTag { name: name.to_string() })
            .ok();
    }
    gid
}

/// Сущности группы по её id.
pub fn group_entities(ecs: &EcsAdapter, gid: u32) -> Vec<specs::Entity> {
    ecs.world
        .read_resource::<GroupInfoResource>()
        .groups
        .get(&gid)
        .map(|g| g.entities.clone())
        .unwrap_or_default()
}

/// Первая сущность группы.
pub fn first_entity(ecs: &EcsAdapter, gid: u32) -> specs::Entity {
    group_entities(ecs, gid)
        .first()
        .copied()
        .expect("у группы нет сущностей")
}

/// Записывает FoodStorage на первую сущность группы.
pub fn set_food(ecs: &mut EcsAdapter, gid: u32, count: i32, max: i32) {
    let entity = first_entity(ecs, gid);
    ecs.world
        .write_storage::<FoodStorage>()
        .insert(entity, FoodStorage { food_count: count, max_food: max })
        .ok();
}

/// Количество групп в мире.
pub fn group_count(ecs: &EcsAdapter) -> usize {
    ecs.world.read_resource::<GroupInfoResource>().groups.len()
}

/// Ищет объект по имени (первая сущность подходящей группы).
pub fn find_object(ecs: &EcsAdapter, name: &str) -> Option<specs::Entity> {
    let tags = ecs.world.read_storage::<ObjectTag>();
    let groups = ecs.world.read_resource::<GroupInfoResource>();
    groups
        .groups
        .values()
        .find(|g| g.name == name)
        .and_then(|g| g.entities.first().copied())
        .filter(|e| tags.get(*e).is_some_and(|t| t.name == name))
}

// ========================================================================
//  Фейковый ввод
// ========================================================================

pub struct FakeInput {
    pub cursor: Option<(f32, f32)>,
    pub pressed: bool,
    pub held: bool,
    pub keys: Vec<KeyCode>,
    pub scroll: (f32, f32),
    pub cursor_delta: (f32, f32),
}

impl Default for FakeInput {
    fn default() -> Self {
        Self {
            cursor: None,
            pressed: false,
            held: false,
            keys: Vec::new(),
            scroll: (0.0, 0.0),
            cursor_delta: (0.0, 0.0),
        }
    }
}

impl FakeInput {
    pub fn new() -> Self {
        Self::default()
    }

    /// Ввод с одиночным нажатием ЛКМ в заданной точке UI.
    pub fn click_ui(x: f32, y: f32, window_size: (f32, f32)) -> Self {
        let mut input = Self::new();
        input.cursor = Some(ui_to_pixels(x, y, window_size));
        input.pressed = true;
        input.held = true;
        input
    }

    /// Ввод с зажатой ЛКМ в заданной точке UI.
    pub fn hold_ui(x: f32, y: f32, window_size: (f32, f32)) -> Self {
        let mut input = Self::new();
        input.cursor = Some(ui_to_pixels(x, y, window_size));
        input.held = true;
        input
    }

    pub fn with_key(mut self, key: KeyCode) -> Self {
        self.keys.push(key);
        self
    }

    pub fn with_scroll(mut self, dy: f32) -> Self {
        self.scroll = (0.0, dy);
        self
    }
}

impl InputSource for FakeInput {
    fn cursor(&self) -> Option<(f32, f32)> { self.cursor }
    fn cursor_diff(&self) -> (f32, f32) { self.cursor_delta }
    fn mouse_pressed(&self, btn: MouseButton) -> bool {
        matches!(btn, MouseButton::Left) && self.pressed
    }
    fn mouse_held(&self, btn: MouseButton) -> bool {
        matches!(btn, MouseButton::Left) && self.held
    }
    fn key_pressed(&self, key: KeyCode) -> bool { self.keys.contains(&key) }
    fn key_held(&self, key: KeyCode) -> bool { self.keys.contains(&key) }
    fn held_control(&self) -> bool { false }
    fn scroll_diff(&self) -> (f32, f32) { self.scroll }
    fn close_requested(&self) -> bool { false }
    fn step(&mut self) {}
    fn end_step(&mut self) {}
    fn process_window_event(&mut self, _event: &WindowEvent) {}
    fn process_device_event(&mut self, _event: &DeviceEvent) {}
}

/// Переводит точку UI (мировые координаты интерфейса, центр экрана в (0,0))
/// в пиксельные координаты окна — обратное к ui::system::ndc_to_ui.
pub fn ui_to_pixels(x: f32, y: f32, window_size: (f32, f32)) -> (f32, f32) {
    let scale = SHADER_SCALE;
    let aspect = window_size.0 / window_size.1;
    let mx = ((x * scale / aspect) + 1.0) * window_size.0 / 2.0;
    let my = (1.0 - y * scale) * window_size.1 / 2.0;
    (mx, my)
}
