// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 EnotCoder

// ========================================================================
//  input_tests — ввод через FakeInput: зум, хит-тесты UI, слайдер,
//  прокрутка магазина, имя мира, затухание у двери и погода.
// ========================================================================

use winit::keyboard::KeyCode;

use crate::core::constants::{ZOOM_MAX, ZOOM_MIN};
use crate::input::camera::handle_zoom;
use crate::save::{unique_world_name, WorldMeta};
use crate::tests::helpers::{ecs_with_map, new_ecs, ui_to_pixels, FakeInput};
use crate::ui::components::{Checkbox, Slider};
use crate::ui::shop::{max_scroll, thumb_height, thumb_y};
use crate::ui::system::{checkbox_clicked, is_clicked, is_inside, slider_drag, slider_thumb_x};

const WINDOW: (f32, f32) = (1280.0, 720.0);

// ========================================================================
//  Зум камеры
// ========================================================================

#[test]
fn handle_zoom_uses_scroll_and_keys_within_bounds() {
    let up = FakeInput::new().with_scroll(1.0);
    let down = FakeInput::new().with_scroll(-1.0);
    let none = FakeInput::new();

    let mid = (ZOOM_MIN + ZOOM_MAX) * 0.5;
    let step = 0.05;

    assert!((handle_zoom(&up, mid, step) - (mid + step)).abs() < 1e-6);
    assert!((handle_zoom(&down, mid, step) - (mid - step)).abs() < 1e-6);
    assert_eq!(handle_zoom(&none, mid, step), mid);

    // Границы: дальше некуда.
    assert_eq!(handle_zoom(&up, ZOOM_MAX, step), ZOOM_MAX);
    assert_eq!(handle_zoom(&down, ZOOM_MIN, step), ZOOM_MIN);
    assert_eq!(handle_zoom(&up, ZOOM_MAX - 0.001, step), ZOOM_MAX);

    // Клавиши K/L — альтернатива колесу.
    let k = FakeInput::new().with_key(KeyCode::KeyK);
    let l = FakeInput::new().with_key(KeyCode::KeyL);
    assert!((handle_zoom(&k, mid, step) - (mid + step)).abs() < 1e-6);
    assert!((handle_zoom(&l, mid, step) - (mid - step)).abs() < 1e-6);
    assert_eq!(handle_zoom(&k, ZOOM_MAX, step), ZOOM_MAX);
    assert_eq!(handle_zoom(&l, ZOOM_MIN, step), ZOOM_MIN);
}

// ========================================================================
//  Хит-тесты UI
// ========================================================================

#[test]
fn is_clicked_requires_press_and_hit_inside_rectangle() {
    let hit = FakeInput::click_ui(0.0, 0.0, WINDOW);
    assert!(is_clicked(&hit, WINDOW, 0.0, 0.0, 1.0, 1.0));
    assert!(!is_clicked(&hit, WINDOW, 5.0, 5.0, 1.0, 1.0));

    let no_press = FakeInput::hold_ui(0.0, 0.0, WINDOW);
    assert!(!is_clicked(&no_press, WINDOW, 0.0, 0.0, 1.0, 1.0));

    // Граница прямоугольника строгая: ровно half_w — уже не попадание.
    assert!(is_inside(0.99, 0.0, 0.0, 0.0, 1.0, 1.0));
    assert!(!is_inside(1.0, 0.0, 0.0, 0.0, 1.0, 1.0));
    assert!(is_inside(0.0, -0.99, 0.0, 0.0, 1.0, 1.0));
    assert!(!is_inside(0.0, 1.0, 0.0, 0.0, 1.0, 1.0));
}

#[test]
fn checkbox_click_and_hover() {
    let checkbox = Checkbox::new(0.0, 0.0, "Test", true);

    let click = FakeInput::click_ui(0.0, 0.0, WINDOW);
    assert!(checkbox_clicked(&checkbox, &click, WINDOW));
    let far_click = FakeInput::click_ui(3.0, 3.0, WINDOW);
    assert!(!checkbox_clicked(&checkbox, &far_click, WINDOW));

    let hover = FakeInput::hold_ui(0.0, 0.0, WINDOW);
    assert!(crate::ui::system::checkbox_hovered(&checkbox, &hover, WINDOW));
    let far = FakeInput::hold_ui(3.0, 3.0, WINDOW);
    assert!(!crate::ui::system::checkbox_hovered(&checkbox, &far, WINDOW));
}

#[test]
fn checkbox_hover_scales_only_the_hovered_box() {
    let mut ecs = new_ecs();
    let mut hovered = Checkbox::new(0.0, 0.0, "Hovered", true);
    let mut other = Checkbox::new(3.0, 0.0, "Other", true);
    let input = FakeInput::hold_ui(0.0, 0.0, WINDOW);

    crate::ui::system::update_checkbox_hover(&mut ecs, &mut hovered, &input, WINDOW, 1.0);
    crate::ui::system::update_checkbox_hover(&mut ecs, &mut other, &input, WINDOW, 1.0);

    assert!(hovered.hover_scale > 1.0, "наведённый чекбокс должен увеличиться");
    assert_eq!(other.hover_scale, 1.0, "соседний чекбокс не должен масштабироваться");
}

#[test]
fn slider_drag_latches_and_releases() {
    let mut slider = Slider::new(0.0, 0.0, "Zoom", 0.0, 1.0, 0.5);

    // Клик вне области не захватывает ползунок.
    let outside = FakeInput::hold_ui(5.0, 5.0, WINDOW);
    assert!(!slider_drag(&mut slider, &outside, WINDOW));
    assert!(!slider.dragging);

    // Нажатие на дорожку захватывает.
    let on_track = FakeInput::hold_ui(0.0, 0.0, WINDOW);
    assert!(slider_drag(&mut slider, &on_track, WINDOW));
    assert!(slider.dragging);

    // Отпускание мыши сбрасывает захват.
    let released = FakeInput::new();
    assert!(!slider_drag(&mut slider, &released, WINDOW));
    assert!(!slider.dragging);
}

#[test]
fn slider_thumb_x_is_linear_and_symmetric() {
    let mut slider = Slider::new(0.0, 0.0, "Zoom", 0.0, 1.0, 0.0);
    let min = slider_thumb_x(&slider);
    slider.value = 1.0;
    let max = slider_thumb_x(&slider);
    slider.value = 0.5;
    let mid = slider_thumb_x(&slider);

    assert!(min < mid && mid < max);
    assert!((mid - slider.x).abs() < 1e-6, "центр значения = центр дорожки");
    assert!((min + max).abs() < 1e-6, "крайние положения симметричны");
}

// ========================================================================
//  Прокрутка магазина
// ========================================================================

#[test]
fn shop_scroll_helpers_are_consistent() {
    assert_eq!(max_scroll(0), 0.0);
    assert_eq!(max_scroll(8), 0.0);
    assert_eq!(max_scroll(9), 1.0);
    assert_eq!(max_scroll(20), 12.0);

    assert_eq!(thumb_height(8), 4.0);
    let h9 = thumb_height(9);
    assert!(h9 < 4.0 && h9 > 3.5);
    assert_eq!(thumb_height(10_000), 0.4, "ползунок не должен стать нулевым");

    // Чем больше прокрутка, тем ниже ползунок; всё в пределах дорожки.
    let total = 20;
    let y0 = thumb_y(0.0, total);
    let y_mid = thumb_y(max_scroll(total) * 0.5, total);
    let y1 = thumb_y(max_scroll(total), total);
    assert!(y0 > y_mid && y_mid > y1);
    for y in [y0, y_mid, y1] {
        assert!((-2.5..=2.0).contains(&y), "ползунок вне дорожки: {y}");
    }
    // При отсутствии прокрутки ползунок по центру дорожки.
    assert_eq!(thumb_y(0.0, 8), -0.25);
}

// ========================================================================
//  Имя мира
// ========================================================================

fn meta(name: &str) -> WorldMeta {
    WorldMeta {
        id: 0,
        name: name.to_string(),
        created: 0,
        updated: 0,
    }
}

#[test]
fn unique_world_name_trims_and_resolves_collisions() {
    assert_eq!(unique_world_name(&[], ""), "World");
    assert_eq!(unique_world_name(&[], "   "), "World");
    assert_eq!(unique_world_name(&[], "  Name  "), "Name");
    assert_eq!(unique_world_name(&[meta("World")], ""), "World(1)");
    assert_eq!(unique_world_name(&[meta("World")], "World"), "World(1)");
    assert_eq!(
        unique_world_name(&[meta("World"), meta("World(1)")], "World"),
        "World(2)"
    );
    assert_eq!(unique_world_name(&[meta("Other")], "World"), "World");
}

// ========================================================================
//  Затухание у двери и погода
// ========================================================================

#[test]
fn doorway_fade_hides_inside_and_ramps_outside() {
    use crate::ecs::adapter::render::doorway_fade;

    assert_eq!(doorway_fade([-0.5, -5.5, 0.0]), 0.0, "внутри проёма");
    assert_eq!(doorway_fade([-0.5, -6.4, 0.0]), 0.0, "у самой кромки проёма");
    // Середина рампы: 0.25 клетки от края при DOOR_FADE_RAMP = 0.5.
    let half = doorway_fade([0.75, -5.5, 0.0]);
    assert!((half - 0.5).abs() < 1e-6, "середина рампы: {half}");
    assert!((doorway_fade([-0.5, -6.75, 0.0]) - 0.5).abs() < 1e-6);

    let a = doorway_fade([0.6, -5.5, 0.0]);
    let b = doorway_fade([0.8, -5.5, 0.0]);
    let c = doorway_fade([1.1, -5.5, 0.0]);
    assert!(a < b && b < c, "затухание должно быть монотонным");
    assert_eq!(doorway_fade([10.0, 10.0, 0.0]), 1.0, "далеко — полностью виден");
}

#[test]
fn weather_rand_is_deterministic_and_bounded() {
    use crate::scenes::game::weather_fx::rand01;

    let mut seed = 0x1234_5678_9abc_def0u64;
    let mut first = Vec::new();
    for _ in 0..100 {
        let v = rand01(&mut seed);
        assert!((0.0..1.0).contains(&v), "rand01 вне [0,1): {v}");
        first.push(v);
    }
    assert!(
        first.windows(2).any(|w| w[0] != w[1]),
        "генератор не должен застревать"
    );

    let mut a = 42u64;
    let mut b = 42u64;
    for _ in 0..10 {
        assert_eq!(rand01(&mut a), rand01(&mut b));
    }
    let mut c = 43u64;
    assert_ne!(rand01(&mut a), rand01(&mut c));
}

#[test]
fn weather_effect_and_bounds_follow_the_map() {
    use crate::ecs::components::Season;
    use crate::scenes::game::weather_fx::{Effect, WeatherFx};

    assert!(matches!(WeatherFx::effect_for(Season::Winter), Some(Effect::Snow)));
    assert!(matches!(WeatherFx::effect_for(Season::Autumn), Some(Effect::Rain)));
    assert!(WeatherFx::effect_for(Season::Spring).is_none());
    assert!(WeatherFx::effect_for(Season::Summer).is_none());

    let empty = new_ecs();
    assert!(WeatherFx::world_bounds(&empty).is_none(), "без карты границ нет");

    let ecs = ecs_with_map();
    let (left, right, bottom, top) = WeatherFx::world_bounds(&ecs).expect("границы карты");
    assert!(left < right && bottom < top);
    assert_eq!(right - left, 53.0, "ширина map.txt");
    assert_eq!(top - bottom, 28.0, "высота map.txt");
    assert!((left - (crate::core::constants::WORLD_OFFSET_X - 0.5)).abs() < 1e-6);
    assert!((top - (crate::core::constants::WORLD_OFFSET_Y + 0.5)).abs() < 1e-6);
}

#[test]
fn ui_coordinates_roundtrip_through_pixels() {
    let (px, py) = ui_to_pixels(1.5, -0.5, WINDOW);
    let (wx, wy) = crate::ui::system::ndc_to_ui(px, py, WINDOW);
    assert!((wx - 1.5).abs() < 1e-4, "x: {wx}");
    assert!((wy - (-0.5)).abs() < 1e-4, "y: {wy}");
}
