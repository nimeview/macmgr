-- Theme switcher — left side of the bar (easy to spot).
local colors = require("colors")
local settings = require("settings")

local theme = sbar.add("item", "theme_switcher", {
  position = "right",
  icon = {
    string = "􀣶",
    font = { size = 15.0 },
    color = colors.yellow,
    padding_left = 8,
    padding_right = 4,
  },
  label = {
    string = "Themes",
    font = {
      family = settings.font.numbers,
      style = settings.font.style_map["Bold"],
      size = 12.0,
    },
    color = colors.white,
    padding_right = 10,
  },
  background = {
    color = colors.bg1,
    border_color = colors.yellow,
    border_width = 1,
    height = 26,
    corner_radius = 8,
  },
  popup = { align = "right", height = 28 },
  padding_right = 6,
  padding_left = 6,
})

sbar.add("bracket", "theme_switcher.bracket", { theme.name }, {
  background = {
    color = colors.bg1,
    border_color = colors.yellow,
    border_width = 1,
    height = 28,
    corner_radius = 9,
  },
})

sbar.add("item", {
  position = "right",
  width = settings.group_paddings,
  label = { drawing = false },
  icon = { drawing = false },
})

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
  if id == "current" then return end
  local name = "theme_switcher.row." .. id
  sbar.add("item", name, {
    position = "popup." .. theme.name,
    width = 200,
    align = "left",
    icon = {
      string = "􀝥",
      color = colors.yellow,
      padding_left = 8,
      padding_right = 6,
    },
    label = {
      string = label,
      padding_right = 10,
      font = { size = 12.0 },
      color = colors.white,
    },
    click_script = "macmgr theme apply '" .. id .. "' --reload >/dev/null 2>&1 &",
  })
  table.insert(popup_items, name)
end

local FALLBACK = {
  { "dim", "Dim / Macchiato" },
  { "bright", "Bright / Mocha" },
  { "latte", "Latte (light)" },
  { "tokyo", "Tokyo Night" },
  { "gruvbox", "Gruvbox" },
  { "nord", "Nord" },
  { "rosepine", "Rosé Pine" },
  { "dracula", "Dracula" },
  { "solarized", "Solarized" },
  { "monokai", "Monokai" },
}

local function show()
  clear_popup()
  theme:set({ popup = { drawing = true } })
  sbar.exec("macmgr theme list 2>/dev/null", function(out)
    local any = false
    if out and out ~= "" then
      for line in out:gmatch("[^\r\n]+") do
        local id = line:match("^(%S+)")
        if id and id ~= "" and id ~= "id" and id ~= "current" then
          local label = line:gsub("^%S+%s+", ""):gsub("%s+$", "")
          if label == "" then label = id end
          add_row(id, label:sub(1, 28))
          any = true
        end
      end
    end
    if not any then
      for _, row in ipairs(FALLBACK) do
        add_row(row[1], row[2])
      end
    end
  end)
end

theme:subscribe("mouse.clicked", function(_)
  local open = theme:query().popup.drawing == "on"
  if open then hide() else show() end
end)

theme:subscribe("mouse.exited.global", function(_)
  -- keep open until click
end)
