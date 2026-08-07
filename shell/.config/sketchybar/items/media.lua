-- Center media player: cover | artist — title · time
-- Popup: horizontal controls + working scrubber ($PERCENTAGE).
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
}

local function fmt_time(sec)
  sec = math.floor(tonumber(sec) or 0)
  if sec < 0 then sec = 0 end
  return string.format("%d:%02d", math.floor(sec / 60), sec % 60)
end

-- Cover / song artwork (left of text)
local cover = sbar.add("item", "media.cover", {
  position = "center",
  drawing = false,
  background = {
    image = {
      string = "media.artwork",
      scale = 0.85,
      corner_radius = 4,
    },
    color = colors.transparent,
    height = 26,
  },
  icon = {
    string = "􀑪",
    font = { size = 14.0 },
    color = colors.magenta,
    drawing = false,
  },
  label = { drawing = false },
  padding_left = 6,
  padding_right = 2,
})

-- Title / artist / elapsed (right of cover)
local player = sbar.add("item", "media.player", {
  position = "center",
  drawing = false,
  updates = true,
  icon = { drawing = false },
  label = {
    string = "No media",
    padding_left = 4,
    padding_right = 10,
    font = { size = 12.0 },
    color = colors.white,
    max_chars = 48,
  },
  background = {
    color = colors.bg1,
    border_color = colors.bg2,
    border_width = 0,
    height = 28,
  },
  popup = {
    align = "center",
    horizontal = true,
    height = 40,
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

-- Popup: stacked title (artist above title via y_offset)
local pop_artist = sbar.add("item", "media.pop.artist", {
  position = "popup." .. player.name,
  icon = { drawing = false },
  width = 0,
  label = {
    string = "—",
    font = { size = 9.0 },
    color = colors.with_alpha(colors.white, 0.6),
    max_chars = 28,
    y_offset = 6,
    width = 0,
  },
  padding_left = 10,
  padding_right = 0,
})

local pop_title = sbar.add("item", "media.pop.title", {
  position = "popup." .. player.name,
  icon = { drawing = false },
  label = {
    string = "—",
    font = { size = 12.0, style = settings.font.style_map["Bold"] },
    color = colors.white,
    max_chars = 26,
    y_offset = -5,
    width = 160,
  },
  padding_left = 0,
  padding_right = 8,
})

local pop_time = sbar.add("item", "media.pop.time", {
  position = "popup." .. player.name,
  icon = { drawing = false },
  label = {
    string = "0:00",
    font = { family = settings.font.numbers, size = 11.0 },
    color = colors.grey,
    width = 72,
    align = "center",
  },
})

-- Scrubber — click_script like volume widget (PERCENTAGE is injected by sketchybar)
local scrubber = sbar.add("slider", 120, {
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
    },
  },
  background = {
    color = colors.bg1,
    height = 2,
    y_offset = -20,
    padding_left = 6,
    padding_right = 6,
  },
  click_script = "$CONFIG_DIR/helpers/media_seek_pct.sh $PERCENTAGE",
})

local function add_btn(name, icon, script)
  return sbar.add("item", name, {
    position = "popup." .. player.name,
    icon = {
      string = icon,
      font = { size = 14.0 },
      padding_left = 8,
      padding_right = 8,
      color = colors.white,
    },
    label = { drawing = false },
    click_script = script,
  })
end

local btn_back15 = add_btn("media.pop.back15", "􀊎", "$CONFIG_DIR/helpers/media_seek.sh -15")
local btn_prev = add_btn("media.pop.prev", icons.media.back, "nowplaying-cli previous")
local btn_pp = add_btn("media.pop.pp", icons.media.play_pause, "nowplaying-cli togglePlayPause")
local btn_next = add_btn("media.pop.next", icons.media.forward, "nowplaying-cli next")
local btn_fwd15 = add_btn("media.pop.fwd15", "􀊐", "$CONFIG_DIR/helpers/media_seek.sh 15")

local function refresh_ui()
  local label = state.title
  if state.artist ~= "" then
    label = state.artist .. " — " .. state.title
  end
  if state.has_track then
    label = label .. "  " .. fmt_time(state.elapsed)
  elseif label == "" then
    label = "No media"
  end

  player:set({
    drawing = state.has_track,
    label = {
      string = label,
      color = state.playing and colors.white or colors.grey,
    },
  })

  -- Cover: song icon always; artwork overlays when MediaRemote provides it
  cover:set({
    drawing = state.has_track,
    icon = {
      string = "􀑪",
      drawing = true,
      color = state.playing and colors.green or colors.magenta,
      font = { size = 15.0 },
      padding_left = 8,
      padding_right = 4,
    },
    background = {
      image = {
        string = "media.artwork",
        scale = 0.85,
        corner_radius = 4,
      },
      color = colors.bg1,
      height = 26,
    },
  })

  pop_title:set({ label = { string = state.title ~= "" and state.title or "—" } })
  pop_artist:set({
    label = {
      string = state.artist ~= "" and state.artist or "—",
      width = "dynamic",
    },
  })
  pop_time:set({
    label = {
      string = fmt_time(state.elapsed) .. "/" .. fmt_time(state.duration),
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
  end
  refresh_ui()
end

local function parse_json_field(out, key)
  local str = out:match('"' .. key .. '"%s*:%s*"(.-)"')
  if str then return str end
  if out:match('"' .. key .. '"%s*:%s*null') then return nil end
  return out:match('"' .. key .. '"%s*:%s*([%d%.]+)')
end

local function poll_now()
  sbar.exec(
    "nowplaying-cli get --json title artist playbackRate elapsedTime duration 2>/dev/null",
    function(out)
      if not out or out == "" then return end
      local title = parse_json_field(out, "title")
      local artist = parse_json_field(out, "artist")
      local rate = tonumber(parse_json_field(out, "playbackRate") or "0") or 0
      local elapsed = tonumber(parse_json_field(out, "elapsedTime") or "0") or 0
      local duration = tonumber(parse_json_field(out, "duration") or "0") or 0
      if type(title) ~= "string" then title = nil end
      if type(artist) ~= "string" then artist = nil end

      if title and title ~= "" then
        apply_track(title, artist, rate > 0, elapsed, duration)
      elseif state.has_track and out:find('"title"%s*:%s*null') then
        apply_track("", "", false, 0, 0)
      end
    end
  )
end

player:subscribe("media_change", function(env)
  local info = env.INFO or {}
  local title = info.title
  local artist = info.artist
  local st = info.state
  if title and title ~= "" and (st == "playing" or st == "paused") then
    apply_track(title, artist, st == "playing", state.elapsed, state.duration)
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
poller:subscribe("routine", poll_now)

local function toggle_popup()
  local open = player:query().popup.drawing == "on"
  if open then
    player:set({ popup = { drawing = false } })
  else
    poll_now()
    player:set({ popup = { drawing = true } })
    -- animate artist width like Felix media
    sbar.animate("tanh", 20, function()
      pop_artist:set({ label = { width = "dynamic" } })
    end)
  end
end

player:subscribe("mouse.clicked", toggle_popup)
cover:subscribe("mouse.clicked", toggle_popup)
player:subscribe("media_toggle_popup", toggle_popup)

sbar.add("item", "media.pad", {
  position = "center",
  width = settings.group_paddings,
  label = { drawing = false },
  icon = { drawing = false },
})

sbar.delay(0.5, poll_now)

local _ = { btn_back15, btn_prev, btn_next, btn_fwd15, btn_pp }
