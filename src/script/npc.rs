use std::cell::RefCell;
use std::path::Path;
use mlua::{Lua, Table, Value};
use specs::WorldExt;
use crate::EcsAdapter;
use crate::npc::ShopperNpc;
use crate::ecs::components::{BusyCassas, Money};
use crate::map::pathfinding::Node;

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
    available: bool,
}

impl NpcScript {
    pub fn new() -> Self {
        let lua = Lua::new();
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
        let ecs_cell = RefCell::new(&mut *ecs);
        let npc_cell = RefCell::new(&mut *npc);
        let npc_cell_ref = &npc_cell;
        let ecs_cell_ref = &ecs_cell;
        let walkable_ref = &walkable;

        lua.scope(|ctx| {
            let tbl = lua.create_table()?;
            tbl.set("state", npc_cell_ref.borrow().state_int())?;
            tbl.set("timer", npc_cell_ref.borrow().state_timer())?;
            tbl.set("food_taken", npc_cell_ref.borrow().is_food_taken())?;
            tbl.set("exiting", npc_cell_ref.borrow().is_exiting())?;
            tbl.set("cassa_x", npc_cell_ref.borrow().cassa_pos().x)?;
            tbl.set("cassa_y", npc_cell_ref.borrow().cassa_pos().y)?;
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

            let despawn = ctx.create_function({
                let npc = npc_cell_ref;
                move |_, (): ()| -> mlua::Result<()> {
                    npc.borrow_mut().request_despawn();
                    Ok(())
                }
            })?;
            tbl.set("despawn", despawn)?;

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

            let start_path = ctx.create_function({
                let npc = npc_cell_ref;
                let walkable = walkable_ref;
                move |_, (tbl, x, y): (Table, i32, i32)| -> mlua::Result<bool> {
                    let mut npc_borrow = npc.borrow_mut();
                    let ok = npc_borrow.start_path(walkable, Node::new(x, y));
                    tbl.set("done", npc_borrow.path_done())?;
                    Ok(ok)
                }
            })?;
            tbl.set("start_path", start_path)?;

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

            let add_money = ctx.create_function({
                let ecs = ecs_cell_ref;
                move |_, n: i32| -> mlua::Result<()> {
                    let ecs_borrow = ecs.borrow_mut();
                    ecs_borrow.world.write_resource::<Money>().0 += n;
                    Ok(())
                }
            })?;
            tbl.set("add_money", add_money)?;

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

            let log = ctx.create_function(|_, msg: String| -> mlua::Result<()> {
                println!("[npc.lua] {}", msg);
                Ok(())
            })?;
            tbl.set("log", log)?;

            let update_fn: mlua::Function = lua.globals().get("npc_update")?;
            update_fn.call::<()>((&tbl, dt))?;

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
