-- ========================================================================
--  npc.lua — поведение покупателя (ShopperNpc)
--  Решения: состояние, тайминги, выбор маршрута.
--  Примитивы движка (walk, take_food, busy...) — Rust, вызываются через npc.
--
--  Состояния (зеркало ShopperState):
--   1 = GoingToRack    2 = GoingToCassa    3 = AtCassa
--   4 = GoingToCandies 5 = AtCandies       6 = GoingToExit
--
--  Полный жизненный цикл покупателя:
--    spawn → идёт к стеллажу (rack) → берёт еду → идёт к кассе (cassa)
--    → платит → (с шансом candy_chance) идёт к конфетам → берёт конфеты
--    → платит → уходит к выходу (exit) → despawn
-- ========================================================================

-- Числа-коды состояний вынесены в константы, чтобы не было магических чисел.
-- Они зеркалят enum ShopperState из Rust (src/npc/mod.rs).
local ST_GOING_TO_RACK = 1
local ST_GOING_TO_CASSA = 2
local ST_AT_CASSA = 3
local ST_GOING_TO_CANDIES = 4
local ST_AT_CANDIES = 5
local ST_GOING_TO_EXIT = 6

-- Конфиг-параметры читаются из глобального CONFIG (scripts/config.lua).
-- Тайминги и экономика живут в одном месте, чтобы балансить без правки кода.
local CASSA_WAIT_SECS = CONFIG.cassa_wait_secs
local CANDY_WAIT_SECS = CONFIG.candy_wait_secs
local MONEY_AT_CASSA = CONFIG.money_at_cassa
local MONEY_AT_CANDY = CONFIG.money_at_candy
local CANDY_CHANCE = CONFIG.candy_chance
local EXIT_X, EXIT_Y = CONFIG.spawn_x, CONFIG.spawn_y

-- Точка входа, вызываемая из Rust каждый кадр (dt — дельта времени в секундах).
-- Возвращает: false = покупатель ещё существует, true/не важно — либо ушёл (despawn),
-- либо попросил завершить обновление на этом кадре.
function npc_update(npc, dt)
    -- 1. Касса удалена: переключиться на другую или уйти
    -- Аварийная ситуация: игрок снёс кассу, пока покупатель шёл к ней.
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

    -- 2. Стеллаж удалён или пуст — деспавн.
    -- (Покупатель ещё не взял еду, его rack исчез/опустошился во время пути.)
    if not npc.food_taken and npc.state == ST_GOING_TO_RACK and not npc:rack_exists() then
        npc:despawn()
        return false
    end

    -- 3. active=false и ещё не взял еду — уходим.
    -- Магазин закрыли (переключатель активен=false): покупатель без корзины уходит.
    if npc.exiting and not npc.food_taken and npc.state == ST_GOING_TO_RACK then
        if npc:start_path(EXIT_X, EXIT_Y) then
            npc.state = ST_GOING_TO_EXIT
        end
    end
    -- 4. active=true и шёл на выход без покупки — возвращаемся к rack.
    -- Магазин переоткрыли раньше, чем покупатель успел скрыться: разворачиваем его.
    if not npc.exiting and not npc.food_taken and npc.state == ST_GOING_TO_EXIT then
        if npc:start_path(npc.rack_x, npc.rack_y) then
            npc.state = ST_GOING_TO_RACK
        end
    end

    if npc.state == ST_GOING_TO_RACK then
        -- Идём к стеллажу (racket). Когда дошли (npc.done) — берём еду.
        if npc.done then
            npc:set_idle()
            if npc.exiting then
                -- Магазин закрыли прямо во время пути — уходим сразу
                npc:despawn()
                return false
            end
            -- Касса могла пропасть, пока мы шли — ищем запасную
            if not npc:cassa_exists() then
                npc:free_busy(npc.cassa_x, npc.cassa_y)
                local cassa = npc:find_any_cassa()
                if cassa ~= nil then
                    npc.cassa_x = cassa.x
                    npc.cassa_y = cassa.y
                else
                    -- Касс больше нет вообще — уходим из магазина
                    if npc:start_path(EXIT_X, EXIT_Y) then
                        npc.state = ST_GOING_TO_EXIT
                    end
                    return false
                end
            end
            -- Касса занята другим покупателем — ждём своей очереди
            if npc:is_busy(npc.cassa_x, npc.cassa_y) then
                return false
            end
            -- Проверяем, осталась ли еда на стеллаже, и снимаем её
            if not npc:take_food() then
                npc:despawn()
                return false
            end
            -- Бронируем кассу за собой и строим путь к ней
            npc:set_busy(npc.cassa_x, npc.cassa_y)
            if npc:start_path(npc.cassa_x, npc.cassa_y) then
                npc.state = ST_GOING_TO_CASSA
            end
            return false
        end
        -- Ещё не дошли — продолжаем идти
        npc:walk(dt)

    elseif npc.state == ST_GOING_TO_CASSA then
        -- Идём к кассе: по прибытии переходим в состояние оплаты с таймером
        if npc.done then
            npc:set_idle()
            npc.state = ST_AT_CASSA
            npc.timer = CASSA_WAIT_SECS
            return false
        end
        npc:walk(dt)

    elseif npc.state == ST_AT_CASSA then
        -- Стоим у кассы: отсчитываем время оплаты, затем решаем, брать ли конфеты
        npc.timer = npc.timer - dt
        npc:set_idle()
        if npc.timer <= 0 then
            -- Оплатили: освобождаем кассу, зачисляем деньги за еду
            npc:free_busy(npc.cassa_x, npc.cassa_y)
            npc:add_money(MONEY_AT_CASSA)
            local want_candy = npc.candy_x ~= nil
                and math.random() < CANDY_CHANCE
                and npc:candy_exists()
            if want_candy then
                -- Покупатель захотел конфеты — идём к ним
                if npc:start_path(npc.candy_x, npc.candy_y) then
                    npc.state = ST_GOING_TO_CANDIES
                else
                    -- Не удалось построить путь — сразу на выход
                    if npc:start_path(EXIT_X, EXIT_Y) then
                        npc.state = ST_GOING_TO_EXIT
                    end
                end
            else
                -- Еда куплена, конфеты не нужны — уходим
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
            -- Дошли до конфет: берём и переходим к оплате за них
            npc:set_idle()
            npc:take_candy()
            npc.state = ST_AT_CANDIES
            npc.timer = CANDY_WAIT_SECS
            return false
        end
        npc:walk(dt)

    elseif npc.state == ST_AT_CANDIES then
        -- Стоим у конфет: таймер, затем деньги за конфеты и выход
        npc.timer = npc.timer - dt
        npc:set_idle()
        if npc.timer <= 0 then
            npc:add_money(MONEY_AT_CANDY)
            if npc:start_path(EXIT_X, EXIT_Y) then
                npc.state = ST_GOING_TO_EXIT
            end
        end

    elseif npc.state == ST_GOING_TO_EXIT then
        -- Идём на выход: в конце способа вызываем walk_to_exit (эффект выхода)
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
