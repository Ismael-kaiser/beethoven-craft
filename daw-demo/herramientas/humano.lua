return {
  nombre = "Humanizar", tipo = "herramienta", grupo = "Notas",
  controles = { { "tiempo", "Tiempo", 0, 0.5, 0.08 }, { "fuerza", "Fuerza", 0, 0.6, 0.15 } },
  notas = function(lista, c)
    for _, n in ipairs(lista) do
      n.paso = math.max(0, n.paso + (math.random() * 2 - 1) * c.tiempo)
      n.fuerza = n.fuerza + (math.random() * 2 - 1) * c.fuerza
    end
    return lista
  end,
}
