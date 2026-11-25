return {
  nombre = "Transitorios", tipo = "efecto", grupo = "Dinámica",
  controles = { { "golpe", "Golpe", -1, 1, 0.4 }, { "cola", "Cola", -1, 1, 0 } },
  sonido = function(e, c)
    local x = maximo(abs(e.izq), abs(e.der))
    local rapido, lento = seguidor(x, 0.001, 0.05), seguidor(x, 0.03, 0.05)
    local d = maximo(rapido - lento, 0) / (rapido + 0.0001)
    local largo = seguidor(x, 0.001, 0.4)
    local s = maximo(largo - rapido, 0) / (largo + 0.0001)
    local g = maximo(1 + d * c.golpe * 2 + s * c.cola * 1.5, 0)
    return e.izq * g, e.der * g
  end,
}
