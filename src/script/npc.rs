// ========================================================================
//  NpcScript — вызов scripts/npc.lua из Rust.
//  На каждый тик состояние NPC упаковывается в Lua-таблицу, скрипт решает,
//  какое действие выполнить (walk / take_food / busy и т.д.), а результат
//  записывается обратно в состояние NPC.
// ========================================================================

use std::cell::RefCell;
use std::path::Path;
use mlua::{Lua, Table, Value};
use specs::WorldExt;
use crate::EcsAdapter;
use crate::npc::ShopperNpc;
use crate::ecs::components::{BusyCassas, Money};
use crate::map::pathfinding::Node;

/// Путь к скрипту NPC; при его отсутствии используется Rust-автомат.
const SCRIPT_PATH: &str = "scripts/npc.lua";

/// Состояние NPC в виде целых чисел для Lua (зеркало ShopperState).
#[allow(dead_code)]
pub const ST_GOING_TO_RACK: i32 = 1;
#[allow(dead_code)]
pub const ST_GOING_TO_CASSA: i32 = 2;
#[allow(dead_code)]
pub const ST_AT_CASSA: i32 = 3;
#[allow(dead_code)]
pub const ST_GOING_TO_CANDIES: i32 = 4;
#[allow(dead_code)]
pub const ST_AT_CANDIES: i32 = 5;
#[allow(dead_code)]
pub const ST_GOING_TO_EXIT: i32 = 6;

/// Движок Lua-скриптов NPC. Создаётся один раз и переиспользуется.
pub struct NpcScript {
    lua: Lua,
    /// true, если scripts/npc.lua найден и загружен.
    available: bool,
}

impl NpcScript {
    /// Инициализирует Lua, публикует баланс (CONFIG) и загружает scripts/npc.lua.
    pub fn new() -> Self {
        let lua = Lua::new();
        // Публикуем баланс, чтобы скрипт мог читать настройки из глобала CONFIG.
        let config = crate::script::config::BalanceConfig::load();
        if let Err(e) = config.publish_to_lua(&lua) {
            eprintln!("[config] ошибка публикации CONFIG в Lua: {e}");
        }
        // Загружаем тело скрипта; при неудаче помечаем движок недоступным.
        let path_exists = Path::new(SCRIPT_PATH).exists();
        if path_exists {
            if let Err(e) = lua.load(std::fs::read_to_string(SCRIPT_PATH).unwrap()).exec() {
                eprintln!("[npc.lua] ошибка загрузки: {e}");
            }
        } else {
            eprintln!("[npc] скрипт {SCRIPT_PATH} не найден — fallback на Rust-автомат");
        }
        NpcScript { lua, available: path_exists }
    }

    /// Выполняет один тик скрипта для конкретного NPC.
    /// Если скрипт недоступен, тик пропускается.
    pub fn update(
        &self,
        npc: &mut ShopperNpc,
        ecs: &mut EcsAdapter,
        dt: f64,
        walkable: &std::collections::HashSet<Node>,
    ) -> mlua::Result<()> {
        if !self.available {
            return Ok(());
        }

        let lua = &self.lua;
        // Обёртки над изменяемыми ссылками для передачи их в замыкания Lua.
        let ecs_cell = RefCell::new(&mut *ecs);
        let npc_cell = RefCell::new(&mut *npc);
        let npc_cell_ref = &npc_cell;
        let ecs_cell_ref = &ecs_cell;
        let walkable_ref = &walkable;

        lua.scope(|ctx| {
            // Таблица npc: входные данные о состоянии NPC для скрипта.
            let tbl = lua.create_table()?;
            tbl.set("state", npc_cell_ref.borrow().state_int())?;
            tbl.set("timer", npc_cell_ref.borrow().state_timer())?;
            tbl.set("food_taken", npc_cell_ref.borrow().is_food_taken())?;
            tbl.set("exiting", npc_cell_ref.borrow().is_exiting())?;
            tbl.set("cassa_x", npc_cell_ref.borrow().cassa_pos().x)?;
            tbl.set("cassa_y", npc_cell_ref.borrow().cassa_pos().y)?;
            // Касса для конфет может отсутствовать — тогда передаём nil.
            match npc_cell_ref.borrow().candy_pos() {
                Some(n) => {
                    tbl.set("candy_x", n.x)?;
                    tbl.set("candy_y", n.y)?;
                }
                None => {
                    tbl.set("candy_x", Value::Nil)?;
                    tbl.set("candy_y", Value::Nil)?;
                }
            }
            tbl.set("rack_x", npc_cell_ref.borrow().rack_pos().x)?;
            tbl.set("rack_y", npc_cell_ref.borrow().rack_pos().y)?;
            tbl.set("done", npc_cell_ref.borrow().path_done())?;

            // Примитив: попросить систему удалить NPC.
            let despawn = ctx.create_function({
                let npc = npc_cell_ref;
                move |_, (): ()| -> mlua::Result<()> {
                    npc.borrow_mut().request_despawn();
                    Ok(())
                }
            })?;
            tbl.set("despawn", despawn)?;

            // Примитив walk: продвинуть NPC по маршруту на dt, вернуть позицию.
            let walk = ctx.create_function({
                let npc = npc_cell_ref;
                let ecs = ecs_cell_ref;
                move |_, (tbl, dt): (Table, f64)| -> mlua::Result<bool> {
                    let mut npc_borrow = npc.borrow_mut();
                    let mut ecs_borrow = ecs.borrow_mut();
                    let npc = &mut *npc_borrow;
                    let ecs = &mut **ecs_borrow;
                    npc.walk_toward(ecs, dt);
                    tbl.set("done", npc.path_done())?;
                    tbl.set("pos_x", npc.pos().0)?;
                    tbl.set("pos_y", npc.pos().1)?;
                    Ok(true)
                }
            })?;
            tbl.set("walk", walk)?;

            // Примитив walk_to_exit: идти к выходу, игнорируя магазин.
            let walk_to_exit = ctx.create_function({
                let npc = npc_cell_ref;
                let ecs = ecs_cell_ref;
                move |_, dt: f64| -> mlua::Result<bool> {
                    let mut npc_borrow = npc.borrow_mut();
                    let mut ecs_borrow = ecs.borrow_mut();
                    let npc = &mut *npc_borrow;
                    let ecs = &mut **ecs_borrow;
                    Ok(npc.walk_to_exit(ecs, dt))
                }
            })?;
            tbl.set("walk_to_exit", walk_to_exit)?;

            // Примитив start_path: начать прокладку маршрута до клетки (x, y).
            let start_path = ctx.create_function({
                let npc = npc_cell_ref;
                let ecs = ecs_cell_ref;
                let walkable = walkable_ref;
                move |_, (tbl, x, y): (Table, i32, i32)| -> mlua::Result<bool> {
                    let mut npc_borrow = npc.borrow_mut();
                    let mut ecs_borrow = ecs.borrow_mut();
                    let ok = npc_borrow.start_path(&mut **ecs_borrow, walkable, Node::new(x, y));
                    tbl.set("done", npc_borrow.path_done())?;
                    Ok(ok)
                }
            })?;
            tbl.set("start_path", start_path)?;

            // Примитив set_idle: перевести NPC в состояние ожидания.
            let set_idle = ctx.create_function({
                let npc = npc_cell_ref;
                let ecs = ecs_cell_ref;
                move |_, (): ()| -> mlua::Result<()> {
                    let npc_borrow = npc.borrow_mut();
                    let mut ecs_borrow = ecs.borrow_mut();
                    npc_borrow.set_idle(&mut **ecs_borrow);
                    Ok(())
                }
            })?;
            tbl.set("set_idle", set_idle)?;

            // Примитив set_walk: перевести NPC в состояние ходьбы.
            let set_walk = ctx.create_function({
                let npc = npc_cell_ref;
                let ecs = ecs_cell_ref;
                move |_, (): ()| -> mlua::Result<()> {
                    let npc_borrow = npc.borrow_mut();
                    let mut ecs_borrow = ecs.borrow_mut();
                    npc_borrow.set_walk(&mut **ecs_borrow);
                    Ok(())
                }
            })?;
            tbl.set("set_walk", set_walk)?;

            // Примитив take_food: взять еду из бокса; сообщает успех операции.
            let take_food = ctx.create_function({
                let npc = npc_cell_ref;
                let ecs = ecs_cell_ref;
                move |_, (): ()| -> mlua::Result<bool> {
                    let mut npc_borrow = npc.borrow_mut();
                    let mut ecs_borrow = ecs.borrow_mut();
                    Ok(npc_borrow.take_food(&mut **ecs_borrow))
                }
            })?;
            tbl.set("take_food", take_food)?;

            // Примитив take_candy: взять конфету; сообщает успех операции.
            let take_candy = ctx.create_function({
                let npc = npc_cell_ref;
                let ecs = ecs_cell_ref;
                move |_, (): ()| -> mlua::Result<bool> {
                    let mut npc_borrow = npc.borrow_mut();
                    let mut ecs_borrow = ecs.borrow_mut();
                    Ok(npc_borrow.take_candy(&mut **ecs_borrow))
                }
            })?;
            tbl.set("take_candy", take_candy)?;

            // Предикаты: проверка наличия нужных объектов в магазине.
            let cassa_exists = ctx.create_function({
                let npc = npc_cell_ref;
                let ecs = ecs_cell_ref;
                move |_, (): ()| -> mlua::Result<bool> {
                    let npc_borrow = npc.borrow();
                    let ecs_borrow = ecs.borrow();
                    Ok(npc_borrow.cassa_exists(&*ecs_borrow))
                }
            })?;
            tbl.set("cassa_exists", cassa_exists)?;

            let rack_exists = ctx.create_function({
                let npc = npc_cell_ref;
                let ecs = ecs_cell_ref;
                move |_, (): ()| -> mlua::Result<bool> {
                    let npc_borrow = npc.borrow();
                    let ecs_borrow = ecs.borrow();
                    Ok(npc_borrow.rack_exists(&*ecs_borrow))
                }
            })?;
            tbl.set("rack_exists", rack_exists)?;

            let candy_exists = ctx.create_function({
                let npc = npc_cell_ref;
                let ecs = ecs_cell_ref;
                move |_, (): ()| -> mlua::Result<bool> {
                    let npc_borrow = npc.borrow();
                    let ecs_borrow = ecs.borrow();
                    Ok(npc_borrow.candy_exists(&*ecs_borrow))
                }
            })?;
            tbl.set("candy_exists", candy_exists)?;

            // Примитив find_any_cassa: найти любую свободную кассу (или nil).
            let find_any_cassa = ctx.create_function({
                let ecs = ecs_cell_ref;
                move |_, (): ()| -> mlua::Result<Value> {
                    let ecs_borrow = ecs.borrow();
                    match ShopperNpc::find_any_cassa(&*ecs_borrow) {
                        Some(n) => Ok(Value::Table(lua.create_table_from([("x", n.x), ("y", n.y)])?)),
                        None => Ok(Value::Nil),
                    }
                }
            })?;
            tbl.set("find_any_cassa", find_any_cassa)?;

            // Примитив reroute_to_cassa: перестроить маршрут к кассе (x, y).
            let reroute_to_cassa = ctx.create_function({
                let npc = npc_cell_ref;
                let ecs = ecs_cell_ref;
                let walkable = walkable_ref;
                move |_, (x, y): (i32, i32)| -> mlua::Result<()> {
                    let mut npc_borrow = npc.borrow_mut();
                    let mut ecs_borrow = ecs.borrow_mut();
                    npc_borrow.reroute_to_cassa(&mut **ecs_borrow, walkable, Node::new(x, y));
                    Ok(())
                }
            })?;
            tbl.set("reroute_to_cassa", reroute_to_cassa)?;

            // Примитив add_money: добавить деньги в кассу игры (ресурс Money).
            let add_money = ctx.create_function({
                let ecs = ecs_cell_ref;
                move |_, n: i32| -> mlua::Result<()> {
                    let ecs_borrow = ecs.borrow_mut();
                    ecs_borrow.world.write_resource::<Money>().0 += n;
                    Ok(())
                }
            })?;
            tbl.set("add_money", add_money)?;

            // Примитивы busy: пометить кассу занятой/свободной и проверить занятость.
            let set_busy = ctx.create_function({
                let ecs = ecs_cell_ref;
                move |_, (x, y): (i32, i32)| -> mlua::Result<()> {
                    let ecs_borrow = ecs.borrow_mut();
                    ecs_borrow.world.write_resource::<BusyCassas>().0.insert((x, y));
                    Ok(())
                }
            })?;
            tbl.set("set_busy", set_busy)?;

            let free_busy = ctx.create_function({
                let ecs = ecs_cell_ref;
                move |_, (x, y): (i32, i32)| -> mlua::Result<()> {
                    let ecs_borrow = ecs.borrow_mut();
                    ecs_borrow.world.write_resource::<BusyCassas>().0.remove(&(x, y));
                    Ok(())
                }
            })?;
            tbl.set("free_busy", free_busy)?;

            let is_busy = ctx.create_function({
                let ecs = ecs_cell_ref;
                move |_, (x, y): (i32, i32)| -> mlua::Result<bool> {
                    let ecs_borrow = ecs.borrow();
                    let busy: std::collections::HashSet<(i32, i32)> =
                        ecs_borrow.world.read_resource::<BusyCassas>().0.clone();
                    Ok(busy.contains(&(x, y)))
                }
            })?;
            tbl.set("is_busy", is_busy)?;

            // Примитив log: вывод отладочных сообщений из скрипта в консоль.
            let log = ctx.create_function(|_, msg: String| -> mlua::Result<()> {
                println!("[npc.lua] {}", msg);
                Ok(())
            })?;
            tbl.set("log", log)?;

            // Вызываем основную функцию скрипта с таблицей состояния и дельтой времени.
            let update_fn: mlua::Function = lua.globals().get("npc_update")?;
            update_fn.call::<()>((&tbl, dt))?;

            // Скрипт записывает новое состояние обратно в таблицу — переносим его в NPC.
            let state = tbl.get::<i32>("state")?;
            let timer = tbl.get::<f64>("timer")?;
            {
                let mut npc_borrow = npc_cell_ref.borrow_mut();
                npc_borrow.set_state_int(state);
                npc_borrow.set_state_timer(timer);
            }
            Ok(())
        })
    }
}

impl Default for NpcScript {
    fn default() -> Self {
        Self::new()
    }
}
