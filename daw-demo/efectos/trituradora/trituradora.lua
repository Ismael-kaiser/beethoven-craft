return {
  nombre = "Trituradora", tipo = "efecto", grupo = "Carácter",
  controles = { { "bits", "Bits", 1, 16, 8 }, { "hz", "Muestreo Hz", 500, 44100, 11025 }, { "mezcla", "Mezcla", 0, 1, 1 } },
  sonido = function(e, c)
    local function f(x) return mezclar(x, escalon(retener(x, c.hz), 2 ^ (c.bits - 1)), c.mezcla) end
    return f(e.izq), f(e.der)
  end,
}
