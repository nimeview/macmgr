return {
  black = 0xff282a36,
  white = 0xfff8f8f2,
  red = 0xffff5555,
  green = 0xff50fa7b,
  blue = 0xff8be9fd,
  yellow = 0xfff1fa8c,
  orange = 0xffffb86c,
  magenta = 0xffff79c6,
  grey = 0xff6272a4,
  transparent = 0x00000000,
  bar = { bg = 0xe0282a36, border = 0xffbd93f9 },
  popup = { bg = 0xe0282a36, border = 0xffff79c6 },
  bg1 = 0xff44475a,
  bg2 = 0xff6272a4,
  with_alpha = function(color, alpha)
    if alpha > 1.0 or alpha < 0.0 then return color end
    return (color & 0x00ffffff) | (math.floor(alpha * 255.0) << 24)
  end,
}
