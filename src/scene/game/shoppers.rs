use std::collections::HashSet;
use specs::Join;
use crate::EcsAdapter;
use crate::map::pathfinding::Node;
use crate::npc::ShopperNpc;
use crate::constants::*;
use crate::ecs::components::{FoodStorage, ObjectTag, Transform};
use specs::WorldExt;
use crate::script::config::BalanceConfig;
use crate::script::npc::NpcScript;

// ========================================================================
//  ShopperManager — менеджер покупателей
// ========================================================================
//  Периодически спавнит покупателей (ShopperNpc) в точке входа магазина,
//  пока есть стеллажи с едой и открытая касса. Направляет их к случайным
//  стеллажам, кассе и выходу. Управляет таймингами спавна и паузой после
//  ухода покупателя.

pub struct ShopperManager {
    shoppers: Vec<ShopperNpc>,
    timer: f64,
    index: usize,
    exit_cooldown: f64,
}

impl ShopperManager {
    pub fn new() -> Self {
        Self {
            shoppers: Vec::new(),
            timer: 0.0,
            index: 0,
            exit_cooldown: 0.0,
        }
    }

    /// Полный сброс (новый уровень/вход в игру): убирает всех покупателей
    pub fn clear(&mut self) {
        self.shoppers.clear();
        self.timer = 0.0;
        self.index = 0;
        self.exit_cooldown = 0.0;
    }

    /// Включает/выключает магазин: при выключении все покупатели уходят
    pub fn set_active(&mut self, active: bool) {
        for shopper in &mut self.shoppers {
            if active {
                // Возвращаем к покупкам только тех, кто ещё не взял товар
                if !shopper.has_taken_food() {
                    shopper.set_exiting(false);
                }
            } else {
                shopper.set_exiting(true);
            }
        }
    }

    /// Создаёт одного нового покупателя: выбирает случайные стеллаж и кассу
    /// с едой, затем спавнит NPC на точке входа
    fn spawn_shopper(&mut self, ecs: &mut EcsAdapter, walkable: &HashSet<Node>) {
        // Собираем позиции всех касс и стеллажей/витрин с остатком еды
        let (all_racks, all_cassas, all_candies) = {
            let tags = ecs.world.read_storage::<ObjectTag>();
            let foods = ecs.world.read_storage::<FoodStorage>();
            let transforms = ecs.world.read_storage::<Transform>();
            let mut racks = Vec::new();
            let mut cassas = Vec::new();
            let mut candies = Vec::new();
            for (tag, transform) in (&tags, &transforms).join() {
                if tag.name == "cassa" {
                    cassas.push(Node::new(transform.position[0] as i32, transform.position[1] as i32));
                }
            }
            for (tag, food, transform) in (&tags, &foods, &transforms).join() {
                if tag.name == "rack" && food.food_count > 0 {
                    // Клетка перед стеллажом, куда подходят покупатели
                    racks.push(Node::new(transform.position[0] as i32, transform.position[1] as i32 + 1));
                }
                if tag.name == "candies" && food.food_count > 0 {
                    candies.push(Node::new(transform.position[0] as i32, transform.position[1] as i32));
                }
            }
            (racks, cassas, candies)
        };
        if !all_racks.is_empty() && !all_cassas.is_empty() {
            // Случайный выбор целей через время системы (играет роль seed)
            let seed = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).unwrap().subsec_nanos();
            let rack = all_racks[(seed as usize) % all_racks.len()];
            let cassa = all_cassas[((seed >> 16) as usize) % all_cassas.len()];
            let candy_pos = if !all_candies.is_empty() {
                Some(all_candies[((seed >> 8) as usize) % all_candies.len()])
            } else {
                None
            };
            let spawn_node = crate::map::shopper_spawn_point();
            // Внешний вид покупателя меняется по кругу (3 набора текстур)
            let tex_set = self.index % 3;
            self.index += 1;
            let (tex_idle, tex_walk_1, tex_walk_2) = match tex_set {
                0 => (TEX_BOB_IDLE, TEX_BOB_WALK_1, TEX_BOB_WALK_2),
                1 => (TEX_PLAYER_IDLE, TEX_PLAYER_WALK_1, TEX_PLAYER_WALK_2),
                _ => (TEX_SASHA_IDLE, TEX_SASHA_WALK_1, TEX_SASHA_WALK_2),
            };
            if let Some(shopper) = ShopperNpc::spawn(ecs, walkable, spawn_node, rack, cassa, candy_pos, tex_idle, tex_walk_1, tex_walk_2) {
                self.shoppers.push(shopper);
            }
        }
    }

    /// Ежекадровая логика: спавн по таймеру до лимита, обновление всех NPC,
    /// удаление ушедших и установка паузы перед следующим спавном
    pub fn tick(&mut self, ecs: &mut EcsAdapter, dt: f64, walkable: &HashSet<Node>, active: bool, config: &BalanceConfig, npc_script: Option<&NpcScript>) {
        self.timer += dt;
        // После ухода покупателя ждём cooldown, затем разрешаем спавн
        if self.exit_cooldown > 0.0 {
            self.exit_cooldown -= dt;
            if self.exit_cooldown <= 0.0 && active && self.shoppers.len() < config.max_shoppers {
                self.timer = 0.0;
                self.spawn_shopper(ecs, walkable);
            }
        }
        // Обычный спавн: магазин активен и прошло время между появлениями
        if active && self.timer >= config.shopper_spawn_interval && self.shoppers.len() < config.max_shoppers && self.exit_cooldown <= 0.0 {
            self.timer = 0.0;
            self.spawn_shopper(ecs, walkable);
        }
        // Обновляем всех покупателей; закончивших покупку удаляем из мира
        let prev_len = self.shoppers.len();
        self.shoppers.retain_mut(|shopper| {
            let done = shopper.update(ecs, dt, walkable, npc_script);
            if done {
                shopper.despawn(ecs);
            }
            !done
        });
        if self.shoppers.len() < prev_len {
            self.exit_cooldown = config.shopper_spawn_cooldown;
        }
    }
}