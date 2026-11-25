return {
  nombre = "Ping-pong", tipo = "efecto", grupo = "Espacio",
  controles = { { "seg", "Tiempo", 0.02, 0.7, 0.3 }, { "repite", "Repite", 0, 0.9, 0.45 }, { "mezcla", "Mezcla", 0, 1, 0.35 }, { "seco", "Seco", 0, 1, 1 } },
  sonido = function(e, c)
    local m = (e.izq + e.der) * 0.5
    local a = retardo(m, c.seg * 2, c.repite, 1.5)
    local primero = retardo(m, c.seg, 0, 0.75)
    local b = primero + retardo(a, c.seg, 0, 0.75)
    return e.izq * c.seco + a * c.mezcla, e.der * c.seco + b * c.mezcla
  end,
}
