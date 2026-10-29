-- Add the sketchybar module to the package cpath
local home = os.getenv("HOME") or ("/Users/" .. os.getenv("USER"))
package.cpath = package.cpath .. ";" .. home .. "/.local/share/sketchybar_lua/?.so"

os.execute("(cd helpers && make)")
