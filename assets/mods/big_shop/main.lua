-- Big Shop: стартовый капитал и новый предмет с собственной текстурой.
mod.log("Big Shop loaded")
mod.set_balance("start_money", 300)

mod.register_item({
  name = "neon_box",
  tab = "regular",
  category = "regular",
  width = 1,
  height = 1,
  texture = "textures/decor/neon_box.png",
  icon = "textures/decor/neon_box.png",
  price = 80,
})

mod.on("object_placed", function(data, dt)
  if data.name == "neon_box" then
    mod.log("Neon box placed at " .. data.x .. "," .. data.y)
  end
end)
