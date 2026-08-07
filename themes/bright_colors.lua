return {
  black = 0xff1e1e2e,
  white = 0xfff5e0dc,
  red = 0xfff38ba8,
  green = 0xffa6e3a1,
  blue = 0xff89b4fa,
  yellow = 0xfff9e2af,
  orange = 0xfffab387,
  magenta = 0xffcba6f7,
  grey = 0xffbac2de,
  transparent = 0x00000000,
  bar = { bg = 0xe01e1e2e, border = 0xffcba6f7 },
  popup = { bg = 0xe01e1e2e, border = 0xff89b4fa },
  bg1 = 0xff313244,
  bg2 = 0xff45475a,
  with_alpha = function(color, alpha)
    if alpha > 1.0 or alpha < 0.0 then return color end
    return (color & 0x00ffffff) | (math.floor(alpha * 255.0) << 24)
  end,
}
