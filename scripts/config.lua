-- ========================================================================
--  config.lua — баланс игры (единый конфиг)
--  Читается движком один раз при старте в BalanceConfig.
--  Покупатель (npc.lua) читает те же значения через глобаль CONFIG.
--  Все значения имеют дефолты в Rust, если файл отсутствует/повреждён.
-- ========================================================================

CONFIG = {
    -- === Спавн покупателей ===
    shopper_spawn_interval = 3.0,
    shopper_spawn_cooldown = 2.0,
    max_shoppers = 15,

    -- === Реген еды в box ===
    food_regen_tick = 1.0,
    food_regen_amount = 1,

    -- === Цикл день/ночь ===
    day_cycle_secs = 120.0,
    day_secs = 55.0,
    night_start_secs = 60.0,
    night_secs = 115.0,
    fade_secs = 5.0,

    -- === Движение/анимация NPC ===
    npc_speed = 3.0,
    walk_anim_interval = 0.3,
    npc_fade_speed = 2.0,
    spawn_x = 0,
    spawn_y = -3,

    -- === Покупатель (ранее локальные константы npc.lua) ===
    cassa_wait_secs = 1.0,
    candy_wait_secs = 3.0,
    money_at_cassa = 5,
    money_at_candy = 1,
    candy_chance = 0.2,

    -- === Вместимости хранилищ ===
    max_food_box = 20,
    max_food_rack = 15,
    max_food_candies = 10,
    candies_start_food = 10,

    -- === Пороги смены текстур еды ===
    box_tex_threshold_1 = 8,
    box_tex_threshold_2 = 12,
}
