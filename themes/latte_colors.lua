return {
  black = 0xffeff1f5,
  white = 0xff4c4f69,
  red = 0xffd20f39,
  green = 0xff40a02b,
  blue = 0xff1e66f5,
  yellow = 0xffdf8e1d,
  orange = 0xfffe640b,
  magenta = 0xff8839ef,
  grey = 0xff6c6f85,
  transparent = 0x00000000,
  bar = { bg = 0xe0eff1f5, border = 0xff8839ef },
  popup = { bg = 0xe0eff1f5, border = 0xff1e66f5 },
  bg1 = 0xffe6e9ef,
  bg2 = 0xffccd0da,
  with_alpha = function(color, alpha)
    if alpha > 1.0 or alpha < 0.0 then return color end
    return (color & 0x00ffffff) | (math.floor(alpha * 255.0) << 24)
  end,
}
