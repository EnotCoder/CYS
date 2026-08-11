// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

use specs::{Component, VecStorage};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

// Позиция сущности в мировых координатах; z задаёт слой (Z_MAP/Z_DECOR/...).
#[derive(Debug)]
pub struct Transform {
    pub position: [f32; 3],
}

impl Component for Transform {
    type Storage = VecStorage<Self>;
}

// SpriteComponent
// Отображаемая графика: путь к текстуре-атласу, выбранный кадр,
// количество кадров по осям атласа, масштаб и прозрачность.
#[derive(Debug)]
pub struct SpriteComponent {
    pub texture_path: Arc<str>,
    pub texture_frame: [i32; 2],
    pub texture_count: [i32; 2],
    pub scale: f32,
    pub alpha: f32,
    pub animated: bool,
    pub frame_paths: Vec<String>,
    pub current_frame: i32,
}

impl Component for SpriteComponent {
    type Storage = VecStorage<Self>;
}

// GroupComponent
// Привязка сущности к группе: один многоклеточный объект (стол, ковёр)
// состоит из нескольких сущностей с одинаковым group_id.
#[derive(Debug)]
pub struct GroupComponent {
    pub group_id: u32,
}

impl Component for GroupComponent {
    type Storage = VecStorage<Self>;
}

// GroupInfo
// Ресурс-реестр всех групп объектов: хранит сущности и размеры группы,
// чтобы можно было искать объекты по позиции и удалять их целиком.
#[derive(Debug, Clone)]
pub struct GroupInfoResource {
    pub groups: HashMap<u32, GroupInfo>,
}

#[derive(Debug, Clone)]
pub struct GroupInfo {
    pub entities: Vec<specs::Entity>,
    pub width: i32,
    pub height: i32,
    pub pos_x: i32,
    pub pos_y: i32,
    pub is_carpet: bool,
}

// Углы поворота сущности вокруг осей (опциональный компонент).
#[derive(Debug)]
pub struct Rotation {
    pub rotation: [f32; 3],
}

impl Component for Rotation {
    type Storage = VecStorage<Self>;
}

// ========================================================================
//  Система компонентов объектов — каждое свойство = отдельный компонент
// ========================================================================

/// Маркер с именем объекта (box, table, rack...)
/// Позволяет отличить тип объекта по компоненту, а не по пути к текстуре.
#[derive(Debug)]
pub struct ObjectTag {
    #[allow(dead_code)]
    pub name: String,
}

impl Component for ObjectTag {
    type Storage = VecStorage<Self>;
}

/// Хранилище еды (для box)
/// Счётчик заполненности: используется для смены текстуры/вместимости.
#[derive(Debug)]
pub struct FoodStorage {
    pub food_count: i32,
    pub max_food: i32,
}

impl Component for FoodStorage {
    type Storage = VecStorage<Self>;
}

// Глобальная сумма еды на всех складах (ресурс мира).
pub struct TotalFood(pub i32);

/// Деньги игрока
/// Глобальный ресурс мира, доступный любой системе.
pub struct Money(pub i32);

/// Ресурс: какие кассы заняты (по позициям)
/// Множество клеток, на которых уже установлены кассы, — запрет повторной установки.
pub struct BusyCassas(pub HashSet<(i32, i32)>);

/// Ресурс: установлен ли подвал (максимум 1 на магазин)
pub struct BasementPlaced(pub bool);

/// Маркер для забора — текстура зависит от соседей
/// Имя определяет вариант забора; итоговый кадр выбирается по соседним
/// заборам (см. update_fence_textures), а не по атласу.
#[derive(Debug)]
pub struct FenceComponent {
    pub name: String,
}

impl Component for FenceComponent {
    type Storage = VecStorage<Self>;
}