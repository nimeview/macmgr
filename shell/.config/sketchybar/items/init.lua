require("items.apple")
require("items.menus")
require("items.spaces")
require("items.front_app")
require("items.calendar")
require("items.media")
require("items.widgets")

-- После создания space.* всегда показываем столы, не меню приложения
sbar.set("/menu\\..*/", { drawing = false })
sbar.set("/space\\..*/", { drawing = true })
sbar.set("front_app", { drawing = true })
