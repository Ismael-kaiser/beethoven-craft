return {
  nombre = "Compresor", tipo = "efecto", grupo = "Dinámica",
  controles = {
    { "umbral", "Umbral dB", -60, 0, -18 }, { "razon", "Razón", 1, 20, 4 },
    { "ataque", "Ataque", 0.0005, 0.2, 0.01 }, { "soltar", "Soltar", 0.02, 1.5, 0.15 },
    { "sube", "Salida dB", -12, 24, 0 }, { "mezcla", "Mezcla", 0, 1, 1 },
  },
  sonido = function(e, c)
    local nivel = seguidor(maximo(abs(e.izq), abs(e.der)), c.ataque, c.soltar)
    local g = maximo(nivel / db(c.umbral), 1) ^ (1 / c.razon - 1) * db(c.sube)
    return mezclar(e.izq, e.izq * g, c.mezcla), mezclar(e.der, e.der * g, c.mezcla)
  end,
}
