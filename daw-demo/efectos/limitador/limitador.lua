-- Mira 3 ms adelante para bajar antes del golpe.
return {
  nombre = "Limitador", tipo = "efecto", grupo = "Dinámica",
  controles = { { "entra", "Entrada dB", 0, 24, 0 }, { "techo", "Techo dB", -12, 0, -0.5 }, { "soltar", "Soltar", 0.01, 1, 0.12 } },
  sonido = function(e, c)
    local a, b, techo = e.izq * db(c.entra), e.der * db(c.entra), db(c.techo)
    local g = techo / maximo(seguidor(maximo(abs(a), abs(b)), 0.0006, c.soltar), techo)
    local function corta(x) return maximo(minimo(x, techo), 0 - techo) end
    return corta(retardo(a, 0.003) * g), corta(retardo(b, 0.003) * g)
  end,
}
