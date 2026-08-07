local icons = require("icons")
local colors = require("colors")
local settings = require("settings")

-- Execute the event provider binary which provides the event "network_update"
-- for the network interface "en0", which is fired every 2.0 seconds.
sbar.exec("killall network_load >/dev/null; $CONFIG_DIR/helpers/event_providers/network_load/bin/network_load en0 network_update 2.0")

local popup_width = 260

local wifi_up = sbar.add("item", "widgets.wifi1", {
  position = "right",
  padding_left = -5,
  width = 0,
  icon = {
    padding_right = 0,
    font = {
      style = settings.font.style_map["Bold"],
      size = 9.0,
    },
    string = icons.wifi.upload,
  },
  label = {
    font = {
      family = settings.font.numbers,
      style = settings.font.style_map["Bold"],
      size = 9.0,
    },
    color = colors.red,
    string = "??? Bps",
  },
  y_offset = 4,
})

local wifi_down = sbar.add("item", "widgets.wifi2", {
  position = "right",
  padding_left = -5,
  icon = {
    padding_right = 0,
    font = {
      style = settings.font.style_map["Bold"],
      size = 9.0,
    },
    string = icons.wifi.download,
  },
  label = {
    font = {
      family = settings.font.numbers,
      style = settings.font.style_map["Bold"],
      size = 9.0,
    },
    color = colors.blue,
    string = "??? Bps",
  },
  y_offset = -4,
})

local wifi = sbar.add("item", "widgets.wifi.padding", {
  position = "right",
  label = { drawing = false },
})

local wifi_bracket = sbar.add("bracket", "widgets.wifi.bracket", {
  wifi.name,
  wifi_up.name,
  wifi_down.name
}, {
  background = { color = colors.bg1 },
  popup = { align = "center", height = 30 }
})

local ssid = sbar.add("item", {
  position = "popup." .. wifi_bracket.name,
  icon = {
    font = { style = settings.font.style_map["Bold"] },
    string = icons.wifi.router,
  },
  width = popup_width,
  align = "center",
  label = {
    font = { size = 15, style = settings.font.style_map["Bold"] },
    max_chars = 22,
    string = "…",
  },
  background = {
    height = 2,
    color = colors.grey,
    y_offset = -15
  }
})

local hostname = sbar.add("item", {
  position = "popup." .. wifi_bracket.name,
  icon = { align = "left", string = "Hostname:", width = popup_width / 2 },
  label = { max_chars = 20, string = "…", width = popup_width / 2, align = "right" }
})

local ip = sbar.add("item", {
  position = "popup." .. wifi_bracket.name,
  icon = { align = "left", string = "IP:", width = popup_width / 2 },
  label = { string = "…", width = popup_width / 2, align = "right" }
})

local mask = sbar.add("item", {
  position = "popup." .. wifi_bracket.name,
  icon = { align = "left", string = "Subnet:", width = popup_width / 2 },
  label = { string = "…", width = popup_width / 2, align = "right" }
})

local router = sbar.add("item", {
  position = "popup." .. wifi_bracket.name,
  icon = { align = "left", string = "Router:", width = popup_width / 2 },
  label = { string = "…", width = popup_width / 2, align = "right" },
})

local nets_header = sbar.add("item", {
  position = "popup." .. wifi_bracket.name,
  icon = { drawing = false },
  width = popup_width,
  label = {
    string = "— Preferred networks —",
    align = "center",
    color = colors.grey,
    font = { size = 11.0 },
  },
})

sbar.add("item", { position = "right", width = settings.group_paddings })

local net_rows = {}

local function clear_net_rows()
  for _, name in ipairs(net_rows) do
    sbar.remove(name)
  end
  net_rows = {}
end

wifi_up:subscribe("network_update", function(env)
  local up_color = (env.upload == "000 Bps") and colors.grey or colors.red
  local down_color = (env.download == "000 Bps") and colors.grey or colors.blue
  wifi_up:set({
    icon = { color = up_color },
    label = { string = env.upload, color = up_color }
  })
  wifi_down:set({
    icon = { color = down_color },
    label = { string = env.download, color = down_color }
  })
end)

wifi:subscribe({"wifi_change", "system_woke"}, function(_)
  sbar.exec("ipconfig getifaddr en0", function(ip_addr)
    local connected = not (ip_addr == "")
    wifi:set({
      icon = {
        string = connected and icons.wifi.connected or icons.wifi.disconnected,
        color = connected and colors.white or colors.red,
      },
    })
  end)
end)

local function hide_details()
  wifi_bracket:set({ popup = { drawing = false } })
  clear_net_rows()
end

local function populate_networks()
  clear_net_rows()
  sbar.exec("networksetup -listpreferredwirelessnetworks en0 2>/dev/null | tail -n +2", function(out)
    if not out or out == "" then return end
    local i = 0
    for line in out:gmatch("[^\r\n]+") do
      local name = line:match("^%s*(.-)%s*$")
      if name and name ~= "" and i < 12 then
        i = i + 1
        local id = "widgets.wifi.net." .. i
        -- Escape for shell: wrap SSID in single quotes, escape existing singles
        local safe = name:gsub("'", "'\\''")
        sbar.add("item", id, {
          position = "popup." .. wifi_bracket.name,
          width = popup_width,
          icon = {
            string = "􀙇",
            padding_left = 8,
            color = colors.blue,
          },
          label = {
            string = name,
            max_chars = 28,
            padding_right = 8,
            font = { size = 12.0 },
          },
          click_script = "networksetup -setairportnetwork en0 '" .. safe .. "' >/tmp/macmgr-wifi.log 2>&1; sketchybar --trigger wifi_change",
        })
        table.insert(net_rows, id)
      end
    end
  end)
end

local function toggle_details()
  local should_draw = wifi_bracket:query().popup.drawing == "off"
  if should_draw then
    wifi_bracket:set({ popup = { drawing = true }})
    sbar.exec("networksetup -getcomputername", function(result)
      hostname:set({ label = result })
    end)
    sbar.exec("ipconfig getifaddr en0", function(result)
      ip:set({ label = result })
    end)
    sbar.exec("ipconfig getsummary en0 | awk -F ' SSID : '  '/ SSID : / {print $2}'", function(result)
      ssid:set({ label = result })
    end)
    sbar.exec("networksetup -getinfo Wi-Fi | awk -F 'Subnet mask: ' '/^Subnet mask: / {print $2}'", function(result)
      mask:set({ label = result })
    end)
    sbar.exec("networksetup -getinfo Wi-Fi | awk -F 'Router: ' '/^Router: / {print $2}'", function(result)
      router:set({ label = result })
    end)
    populate_networks()
  else
    hide_details()
  end
end

wifi_up:subscribe("mouse.clicked", toggle_details)
wifi_down:subscribe("mouse.clicked", toggle_details)
wifi:subscribe("mouse.clicked", toggle_details)
wifi:subscribe("mouse.exited.global", hide_details)

local function copy_label_to_clipboard(env)
  local label = sbar.query(env.NAME).label.value
  sbar.exec("echo \"" .. label .. "\" | pbcopy")
  sbar.set(env.NAME, { label = { string = icons.clipboard, align="center" } })
  sbar.delay(1, function()
    sbar.set(env.NAME, { label = { string = label, align = "right" } })
  end)
end

ssid:subscribe("mouse.clicked", copy_label_to_clipboard)
hostname:subscribe("mouse.clicked", copy_label_to_clipboard)
ip:subscribe("mouse.clicked", copy_label_to_clipboard)
mask:subscribe("mouse.clicked", copy_label_to_clipboard)
router:subscribe("mouse.clicked", copy_label_to_clipboard)

-- silence unused
local _ = nets_header
