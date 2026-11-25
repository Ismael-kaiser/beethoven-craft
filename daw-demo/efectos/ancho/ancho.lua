return {
  nombre = "Ancho estéreo", tipo = "efecto", grupo = "Espacio",
  controles = { { "ancho", "Ancho", 0, 2, 1.3 }, { "mono", "Mono hasta Hz", 20, 400, 120 } },
  sonido = function(e, c)
    local m, l = (e.izq + e.der) * 0.5, (e.izq - e.der) * 0.5
    l = pasaaltos(l, c.mono, 0) * c.ancho
    return m + l, m - l
  end,
}
