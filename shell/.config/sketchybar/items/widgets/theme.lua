-- Theme switcher in the bar — cycles / picks macmgr color themes.
local colors = require("colors")
local settings = require("settings")

local theme = sbar.add("item", "widgets.theme", {
  position = "right",
  icon = {
    string = "􀆿",
    font = { style = settings.font.style_map["Regular"], size = 14.0 },
    color = colors.yellow,
  },
  label = {
    string = "theme",
    font = { family = settings.font.numbers, size = 11.0 },
    color = colors.white,
  },
  background = { color = colors.bg1 },
  popup = { align = "right", height = 28 },
})

sbar.add("bracket", "widgets.theme.bracket", { theme.name }, {
  background = { color = colors.bg1 },
})

sbar.add("item", { position = "right", width = settings.group_paddings })

local popup_items = {}

local function clear_popup()
  for _, name in ipairs(popup_items) do
    sbar.remove(name)
  end
  popup_items = {}
end

local function hide()
  theme:set({ popup = { drawing = false } })
  clear_popup()
end

local function add_row(id, label)
  local name = "widgets.theme.row." .. id
  sbar.add("item", name, {
    position = "popup." .. theme.name,
    width = 180,
    align = "left",
    icon = { drawing = false },
    label = {
      string = label,
      padding_left = 10,
      padding_right = 10,
      font = { size = 12.0 },
    },
    click_script = "macmgr theme apply " .. id .. " --reload >/dev/null 2>&1 &",
  })
  table.insert(popup_items, name)
end

local function show()
  clear_popup()
  theme:set({ popup = { drawing = true } })
  sbar.exec("macmgr theme list 2>/dev/null", function(out)
    if not out or out == "" then
      add_row("dim", "dim (fallback)")
      add_row("bright", "bright (fallback)")
      return
    end
    for line in out:gmatch("[^\r\n]+") do
      local id = line:match("^(%S+)")
      if id and id ~= "" and not id:find("─") and id ~= "id" then
        local label = line:gsub("^%S+%s+", ""):gsub("%s+$", "")
        if label == "" then label = id end
        add_row(id, id .. " · " .. label:sub(1, 24))
      end
    end
  end)
end

theme:subscribe("mouse.clicked", function(_)
  local open = theme:query().popup.drawing == "on"
  if open then hide() else show() end
end)

theme:subscribe("mouse.exited.global", hide)
