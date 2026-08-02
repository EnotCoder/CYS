-- ========================================================================
--  npc.lua — поведение покупателя (ShopperNpc)
--  Решения: состояние, тайминги, выбор маршрута.
--  Примитивы движка (walk, take_food, busy...) — Rust, вызываются через npc.
--
--  Состояния (зеркало ShopperState):
--   1 = GoingToRack    2 = GoingToCassa    3 = AtCassa
--   4 = GoingToCandies 5 = AtCandies       6 = GoingToExit
-- ========================================================================

local ST_GOING_TO_RACK = 1
local ST_GOING_TO_CASSA = 2
local ST_AT_CASSA = 3
local ST_GOING_TO_CANDIES = 4
local ST_AT_CANDIES = 5
local ST_GOING_TO_EXIT = 6

-- Конфиг-параметры (вынесены из констант Rust)
local CASSA_WAIT_SECS = 1.0
local CANDY_WAIT_SECS = 3.0
local MONEY_AT_CASSA = 5
local MONEY_AT_CANDY = 1
local CANDY_CHANCE = 0.2   -- 20%

-- Точка выхода (спавн)
local EXIT_X, EXIT_Y = 0, -3

function npc_update(npc, dt)
    -- 1. Касса удалена: переключиться на другую или уйти
    if npc.state ~= ST_GOING_TO_EXIT and not npc:cassa_exists() then
        local cassa = npc:find_any_cassa()
        if cassa ~= nil then
            if npc.food_taken then
                npc:reroute_to_cassa(cassa.x, cassa.y)
            else
                npc.cassa_x = cassa.x
                npc.cassa_y = cassa.y
            end
        else
            -- Касс нет — уходим
            if npc:start_path(EXIT_X, EXIT_Y) then
                npc.state = ST_GOING_TO_EXIT
                return false
            end
            npc:despawn()
            return false
        end
    end

    -- 2. Стеллаж удалён или пуст — деспавн
    if not npc.food_taken and npc.state == ST_GOING_TO_RACK and not npc:rack_exists() then
        npc:despawn()
        return false
    end

    -- 3. active=false и ещё не взял еду — уходим
    if npc.exiting and not npc.food_taken and npc.state == ST_GOING_TO_RACK then
        if npc:start_path(EXIT_X, EXIT_Y) then
            npc.state = ST_GOING_TO_EXIT
        end
    end
    -- 4. active=true и шёл на выход без покупки — возвращаемся к rack
    if not npc.exiting and not npc.food_taken and npc.state == ST_GOING_TO_EXIT then
        if npc:start_path(npc.rack_x, npc.rack_y) then
            npc.state = ST_GOING_TO_RACK
        end
    end

    if npc.state == ST_GOING_TO_RACK then
        if npc.done then
            npc:set_idle()
            if npc.exiting then
                npc:despawn()
                return false
            end
            if not npc:cassa_exists() then
                npc:free_busy(npc.cassa_x, npc.cassa_y)
                local cassa = npc:find_any_cassa()
                if cassa ~= nil then
                    npc.cassa_x = cassa.x
                    npc.cassa_y = cassa.y
                else
                    if npc:start_path(EXIT_X, EXIT_Y) then
                        npc.state = ST_GOING_TO_EXIT
                    end
                    return false
                end
            end
            if npc:is_busy(npc.cassa_x, npc.cassa_y) then
                return false
            end
            if not npc:take_food() then
                npc:despawn()
                return false
            end
            npc:set_busy(npc.cassa_x, npc.cassa_y)
            if npc:start_path(npc.cassa_x, npc.cassa_y) then
                npc.state = ST_GOING_TO_CASSA
            end
            return false
        end
        npc:walk(dt)

    elseif npc.state == ST_GOING_TO_CASSA then
        if npc.done then
            npc:set_idle()
            npc.state = ST_AT_CASSA
            npc.timer = CASSA_WAIT_SECS
            return false
        end
        npc:walk(dt)

    elseif npc.state == ST_AT_CASSA then
        npc.timer = npc.timer - dt
        npc:set_idle()
        if npc.timer <= 0 then
            npc:free_busy(npc.cassa_x, npc.cassa_y)
            npc:add_money(MONEY_AT_CASSA)
            local want_candy = npc.candy_x ~= nil
                and math.random() < CANDY_CHANCE
                and npc:candy_exists()
            if want_candy then
                if npc:start_path(npc.candy_x, npc.candy_y) then
                    npc.state = ST_GOING_TO_CANDIES
                else
                    if npc:start_path(EXIT_X, EXIT_Y) then
                        npc.state = ST_GOING_TO_EXIT
                    end
                end
            else
                if npc:start_path(EXIT_X, EXIT_Y) then
                    npc.state = ST_GOING_TO_EXIT
                end
            end
        end

    elseif npc.state == ST_GOING_TO_CANDIES then
        -- Конфеты удалены или кончились — пропускаем и уходим
        if not npc:candy_exists() then
            if npc:start_path(EXIT_X, EXIT_Y) then
                npc.state = ST_GOING_TO_EXIT
            end
            return false
        end
        if npc.done then
            npc:set_idle()
            npc:take_candy()
            npc.state = ST_AT_CANDIES
            npc.timer = CANDY_WAIT_SECS
            return false
        end
        npc:walk(dt)

    elseif npc.state == ST_AT_CANDIES then
        npc.timer = npc.timer - dt
        npc:set_idle()
        if npc.timer <= 0 then
            npc:add_money(MONEY_AT_CANDY)
            if npc:start_path(EXIT_X, EXIT_Y) then
                npc.state = ST_GOING_TO_EXIT
            end
        end

    elseif npc.state == ST_GOING_TO_EXIT then
        if npc.done then
            if npc:walk_to_exit(dt) then
                npc:despawn()
                return false
            end
            return false
        end
        npc:walk(dt)
    end

    return false
end
