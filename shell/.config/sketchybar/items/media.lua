-- Center media player with full popup (controls + scrubber).
-- Stays visible while paused; hides only when nothing is loaded.
-- Requires: brew install nowplaying-cli
local icons = require("icons")
local colors = require("colors")
local settings = require("settings")

sbar.add("event", "media_toggle_popup")

local state = {
  title = "",
  artist = "",
  elapsed = 0,
  duration = 0,
  playing = false,
  has_track = false,
  popup_open = false,
}

local function fmt_time(sec)
  sec = math.floor(tonumber(sec) or 0)
  if sec < 0 then sec = 0 end
  local m = math.floor(sec / 60)
  local s = sec % 60
  return string.format("%d:%02d", m, s)
end

local cover = sbar.add("item", "media.cover", {
  position = "center",
  drawing = false,
  background = {
    image = {
      string = "media.artwork",
      scale = 0.95,
      corner_radius = 5,
    },
    color = colors.transparent,
    height = 28,
  },
  label = { drawing = false },
  icon = { drawing = false },
  padding_right = 4,
})

local player = sbar.add("item", "media.player", {
  position = "center",
  drawing = false,
  updates = true,
  icon = {
    string = icons.media.play_pause,
    padding_left = 8,
    padding_right = 6,
    color = colors.magenta,
    font = { size = 14.0 },
  },
  label = {
    string = "No media",
    padding_right = 10,
    font = { size = 12.0 },
    color = colors.white,
    max_chars = 40,
  },
  background = {
    color = colors.bg1,
    border_color = colors.bg2,
    border_width = 1,
    height = 28,
    corner_radius = 8,
  },
  popup = {
    align = "center",
    horizontal = false,
  },
})

sbar.add("bracket", "media.bracket", { cover.name, player.name }, {
  background = {
    color = colors.bg1,
    border_color = colors.bg2,
    border_width = 1,
    height = 30,
    corner_radius = 10,
  },
})

-- Popup panel (vertical)
local pop_title = sbar.add("item", "media.pop.title", {
  position = "popup." .. player.name,
  icon = { drawing = false },
  width = 280,
  align = "center",
  label = {
    string = "—",
    font = { size = 13.0, style = settings.font.style_map["Bold"] },
    color = colors.white,
    max_chars = 36,
    padding_left = 12,
    padding_right = 12,
  },
})

local pop_artist = sbar.add("item", "media.pop.artist", {
  position = "popup." .. player.name,
  icon = { drawing = false },
  width = 280,
  align = "center",
  label = {
    string = "—",
    font = { size = 11.0 },
    color = colors.grey,
    max_chars = 40,
    padding_left = 12,
    padding_right = 12,
  },
})

local pop_time = sbar.add("item", "media.pop.time", {
  position = "popup." .. player.name,
  icon = { drawing = false },
  width = 280,
  align = "center",
  label = {
    string = "0:00 / 0:00",
    font = { family = settings.font.numbers, size = 11.0 },
    color = colors.white,
  },
})

local scrubber = sbar.add("slider", 240, {
  position = "popup." .. player.name,
  slider = {
    percentage = 0,
    highlight_color = colors.magenta,
    background = {
      height = 6,
      corner_radius = 3,
      color = colors.bg2,
    },
    knob = {
      string = "􀀁",
      drawing = true,
      color = colors.white,
    },
  },
  background = {
    color = colors.transparent,
    padding_left = 20,
    padding_right = 20,
  },
})

local function add_btn(name, icon, script)
  return sbar.add("item", name, {
    position = "popup." .. player.name,
    icon = {
      string = icon,
      font = { size = 16.0 },
      padding_left = 14,
      padding_right = 14,
      color = colors.white,
    },
    label = { drawing = false },
    width = 56,
    align = "center",
    click_script = script,
  })
end

-- Control row (separate items stacked vertically is fine; use wide click targets)
local btn_back15 = add_btn("media.pop.back15", "􀊎", "$CONFIG_DIR/helpers/media_seek.sh -15")
local btn_prev = add_btn("media.pop.prev", icons.media.back, "nowplaying-cli previous")
local btn_pp = add_btn("media.pop.pp", icons.media.play_pause, "nowplaying-cli togglePlayPause")
local btn_next = add_btn("media.pop.next", icons.media.forward, "nowplaying-cli next")
local btn_fwd15 = add_btn("media.pop.fwd15", "􀊐", "$CONFIG_DIR/helpers/media_seek.sh 15")

-- Make controls sit as a horizontal-looking group via shared y — sketchybar vertical
-- popup stacks them; wrap in a note: users get full panel under the bar item.

local function refresh_ui()
  local label = state.title
  if state.artist ~= "" then
    label = state.artist .. " — " .. state.title
  end
  if label == "" then label = "No media" end

  local icon = state.playing and "􀊆" or icons.media.play_pause
  local icon_color = state.playing and colors.green or colors.magenta

  player:set({
    drawing = state.has_track,
    icon = { string = icon, color = icon_color },
    label = {
      string = label,
      color = state.playing and colors.white or colors.grey,
    },
  })
  cover:set({ drawing = state.has_track })

  pop_title:set({ label = { string = state.title ~= "" and state.title or "—" } })
  pop_artist:set({ label = { string = state.artist ~= "" and state.artist or "—" } })
  pop_time:set({
    label = {
      string = fmt_time(state.elapsed) .. " / " .. fmt_time(state.duration),
    },
  })

  local pct = 0
  if state.duration > 0 then
    pct = math.floor((state.elapsed / state.duration) * 100 + 0.5)
    if pct > 100 then pct = 100 end
  end
  scrubber:set({ slider = { percentage = pct } })

  btn_pp:set({
    icon = {
      string = state.playing and "􀊆" or "􀊄",
      color = colors.white,
    },
  })
end

local function apply_track(title, artist, playing, elapsed, duration)
  title = title or ""
  artist = artist or ""
  if title == "null" then title = "" end
  if artist == "null" then artist = "" end

  local has = title ~= ""
  state.title = title
  state.artist = artist
  state.playing = playing and true or false
  state.elapsed = tonumber(elapsed) or 0
  state.duration = tonumber(duration) or 0
  state.has_track = has

  if not has then
    player:set({ popup = { drawing = false } })
    state.popup_open = false
  end
  refresh_ui()
end

local function parse_json_field(out, key)
  local str = out:match('"' .. key .. '"%s*:%s*"(.-)"')
  if str then return str end
  if out:match('"' .. key .. '"%s*:%s*null') then return nil end
  local num = out:match('"' .. key .. '"%s*:%s*([%d%.]+)')
  return num
end

local function poll_now()
  sbar.exec(
    "nowplaying-cli get --json title artist appName playbackRate elapsedTime duration 2>/dev/null",
    function(out)
      if not out or out == "" then
        -- keep last track visible if we already have one (paused apps sometimes return empty)
        return
      end
      local title = parse_json_field(out, "title")
      local artist = parse_json_field(out, "artist")
      local rate = tonumber(parse_json_field(out, "playbackRate") or "0") or 0
      local elapsed = tonumber(parse_json_field(out, "elapsedTime") or "0") or 0
      local duration = tonumber(parse_json_field(out, "duration") or "0") or 0
      if type(title) ~= "string" then title = nil end
      if type(artist) ~= "string" then artist = nil end

      if title and title ~= "" then
        apply_track(title, artist, rate > 0, elapsed, duration)
      elseif state.has_track then
        -- no title in response but we had a track: treat as stopped only if clearly empty object
        if out:find('"title"%s*:%s*null') then
          apply_track("", "", false, 0, 0)
        end
      end
    end
  )
end

player:subscribe("media_change", function(env)
  local info = env.INFO or {}
  local title = info.title
  local artist = info.artist
  local st = info.state
  local playing = (st == "playing")
  local paused = (st == "paused")
  if title and title ~= "" and (playing or paused) then
    apply_track(title, artist, playing, state.elapsed, state.duration)
    poll_now()
  elseif st == "stopped" or st == "none" then
    apply_track("", "", false, 0, 0)
  end
end)

local poller = sbar.add("item", "media.poller", {
  drawing = false,
  updates = true,
  update_freq = 1,
})

poller:subscribe("routine", function(_)
  poll_now()
end)

local function toggle_popup()
  if not state.has_track then
    poll_now()
  end
  local open = player:query().popup.drawing == "on"
  if open then
    player:set({ popup = { drawing = false } })
    state.popup_open = false
  else
    poll_now()
    player:set({ popup = { drawing = true } })
    state.popup_open = true
  end
end

player:subscribe("mouse.clicked", function(_)
  toggle_popup()
end)

cover:subscribe("mouse.clicked", function(_)
  toggle_popup()
end)

-- Don't auto-close on mouse exit while using the scrubber
player:subscribe("mouse.exited.global", function(_)
  -- keep open; click again to close
end)

scrubber:subscribe("mouse.clicked", function(env)
  local pct = env.PERCENTAGE
  if pct then
    sbar.exec("$CONFIG_DIR/helpers/media_seek_pct.sh " .. tostring(pct))
    sbar.delay(0.2, poll_now)
  end
end)

-- External hotkey / script can open panel: sketchybar --trigger media_toggle_popup
player:subscribe("media_toggle_popup", function(_)
  toggle_popup()
end)

sbar.add("item", "media.pad", {
  position = "center",
  width = settings.group_paddings,
  label = { drawing = false },
  icon = { drawing = false },
})

-- Initial poll
sbar.delay(0.5, poll_now)

-- silence unused locals (buttons kept for click_script)
local _ = { btn_back15, btn_prev, btn_next, btn_fwd15 }
