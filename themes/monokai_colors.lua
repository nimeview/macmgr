return {
  black = 0xff272822,
  white = 0xfff8f8f2,
  red = 0xfff92672,
  green = 0xffa6e22e,
  blue = 0xff66d9ef,
  yellow = 0xffe6db74,
  orange = 0xfffd971f,
  magenta = 0xffae81ff,
  grey = 0xff75715e,
  transparent = 0x00000000,
  bar = { bg = 0xe0272822, border = 0xfff92672 },
  popup = { bg = 0xe0272822, border = 0xff66d9ef },
  bg1 = 0xff3e3d32,
  bg2 = 0xff49483e,
  with_alpha = function(color, alpha)
    if alpha > 1.0 or alpha < 0.0 then return color end
    return (color & 0x00ffffff) | (math.floor(alpha * 255.0) << 24)
  end,
}
