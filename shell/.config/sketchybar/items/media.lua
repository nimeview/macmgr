-- Center media player (Spotify / Apple Music / Yandex Music via MediaRemote).
-- Requires: brew install nowplaying-cli
local icons = require("icons")
local colors = require("colors")
local settings = require("settings")

local function allowed_app(app)
  if not app or app == "" then return false end
  local lower = string.lower(app)
  if lower:find("yandex", 1, true) then return true end
  if lower:find("яндекс", 1, true) then return true end
  local whitelist = {
    ["Spotify"] = true,
    ["Music"] = true,
    ["Yandex Music"] = true,
    ["Яндекс Музыка"] = true,
    ["Яндекс.Музыка"] = true,
  }
  return whitelist[app] == true
end

local player = sbar.add("item", "media.player", {
  position = "center",
  drawing = false,
  updates = true,
  icon = {
    string = icons.media.play_pause,
    padding_left = 8,
    padding_right = 6,
    color = colors.magenta,
  },
  label = {
    string = "Not playing",
    padding_right = 10,
    font = { size = 12.0 },
    color = colors.white,
    max_chars = 42,
  },
  background = {
    color = colors.bg1,
    border_color = colors.bg2,
    border_width = 1,
    height = 26,
    corner_radius = 8,
  },
  popup = {
    align = "center",
    horizontal = true,
    height = 36,
  },
})

local cover = sbar.add("item", "media.cover", {
  position = "center",
  drawing = false,
  background = {
    image = {
      string = "media.artwork",
      scale = 0.9,
      corner_radius = 4,
    },
    color = colors.transparent,
    height = 26,
  },
  label = { drawing = false },
  icon = { drawing = false },
})

local function add_ctrl(icon, script)
  sbar.add("item", {
    position = "popup." .. player.name,
    icon = {
      string = icon,
      font = { size = 14.0 },
      padding_left = 10,
      padding_right = 10,
    },
    label = { drawing = false },
    click_script = script,
  })
end

add_ctrl("􀊎", "$CONFIG_DIR/helpers/media_seek.sh -15")
add_ctrl(icons.media.back, "nowplaying-cli previous")
add_ctrl(icons.media.play_pause, "nowplaying-cli togglePlayPause")
add_ctrl(icons.media.forward, "nowplaying-cli next")
add_ctrl("􀊐", "$CONFIG_DIR/helpers/media_seek.sh 15")

local function set_playing(title, artist, drawing)
  local label = title or ""
  if artist and artist ~= "" and artist ~= "null" then
    label = artist .. " — " .. label
  end
  if label == "" then label = "Playing…" end
  player:set({
    drawing = drawing,
    label = { string = label },
  })
  cover:set({ drawing = drawing })
  if not drawing then
    player:set({ popup = { drawing = false } })
  end
end

player:subscribe("media_change", function(env)
  local info = env.INFO or {}
  local app = info.app
  if not allowed_app(app) then
    -- Still show if MediaRemote reports playing and title exists (Yandex variants)
    if info.state == "playing" and info.title and info.title ~= "" then
      set_playing(info.title, info.artist, true)
    else
      set_playing(nil, nil, false)
    end
    return
  end
  local drawing = (info.state == "playing" or info.state == "paused")
  set_playing(info.title, info.artist, drawing and (info.title ~= nil and info.title ~= ""))
end)

-- Poll fallback for apps that don't always fire media_change (e.g. some Yandex builds)
local poller = sbar.add("item", "media.poller", {
  drawing = false,
  updates = true,
  update_freq = 3,
})

poller:subscribe("routine", function(_)
  sbar.exec("nowplaying-cli get --json title artist appName playbackRate 2>/dev/null", function(out)
    if not out or out == "" then return end
    -- crude parse without full JSON lib
    local title = out:match('"title"%s*:%s*"(.-)"') or out:match('"title"%s*:%s*null')
    local artist = out:match('"artist"%s*:%s*"(.-)"')
    local app = out:match('"appName"%s*:%s*"(.-)"')
    local rate = tonumber(out:match('"playbackRate"%s*:%s*([%d%.]+)')) or 0
    if title == "null" then title = nil end
    if artist == "null" then artist = nil end
    if title and title ~= "" and (allowed_app(app) or (app and app:lower():find("yandex")) or rate > 0) then
      set_playing(title, artist, true)
    end
  end)
end)

player:subscribe("mouse.clicked", function(_)
  player:set({ popup = { drawing = "toggle" } })
end)

player:subscribe("mouse.exited.global", function(_)
  player:set({ popup = { drawing = false } })
end)

cover:subscribe("mouse.clicked", function(_)
  player:set({ popup = { drawing = "toggle" } })
end)

-- Keep spacing consistent when empty
sbar.add("item", "media.pad", {
  position = "center",
  width = settings.group_paddings,
  label = { drawing = false },
  icon = { drawing = false },
})
