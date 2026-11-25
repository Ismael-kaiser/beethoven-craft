local equipo = incluir("equipo")

-- Congelar: detiene el sonido en un instante y lo deja sonando, por frecuencias.
return {
  nombre = { es = "Congelador espectral", en = "Spectral freeze" }, tipo = "efecto", sostiene = true, grupo = "Espacio",
  controles = { { "hielo", { es = "Congelar", en = "Freeze" }, 0, 1, 0 }, { "difuso", { es = "Difuminar", en = "Blur" }, 0, 0.995, 0.6 }, { "mezcla", "Mezcla", 0, 1, 0.6 } },
  presets = { { nombre = { es = "Nube", en = "Cloud" }, difuso = 0.95, mezcla = 0.7 }, { nombre = { es = "Detenido", en = "Frozen" }, hielo = 1, difuso = 0.8, mezcla = 1 } },
  vivo = function(v)
    local n = 2048
    local mitad = n // 2 + 1
    local guarda = { {}, {} }
    for l = 1, 2 do for i = 1, mitad do guarda[l][i] = 0.0 end end
    return espectral(n, function(re, im, l, c, m)
      local g = guarda[l]
      local quieto = c.hielo > 0.5
      local k = c.difuso
      for i = 1, mitad do
        local a, b = re[i], im[i]
        local x = math.sqrt(a * a + b * b)
        if not quieto then g[i] = x > g[i] and x + (g[i] - x) * k * 0.3 or x + (g[i] - x) * k end
        -- La fase al azar: suena parejo, sin el golpe original.
        local f = math.random() * 6.2832
        local ya, yb = g[i] * math.cos(f), g[i] * math.sin(f)
        re[i], im[i] = a + (ya - a) * c.mezcla, b + (yb - b) * c.mezcla
        if i > 1 and i < mitad then
          local j = n - i + 2
          re[j], im[j] = re[i], -im[i]
        end
      end
    end)
  end,
  interfaz = equipo.rack{ tinte = "#3c6a7c", color = "#b8f4ff", metal = "#b4b7bd", grafica = false, aguja = function(c, m) return equipo.vu(m.nivel), "VU" end,
    arriba = function(d, c, m, x, y, w, h)
      d.lista(x + 200, y + 30, 220, 36, "hielo", m.idioma == "en" and { "Flow", "Freeze" } or { "Fluye", "Congelar" }, "#b8f4ff")
    end, claves = { "difuso", "mezcla" } },
}
