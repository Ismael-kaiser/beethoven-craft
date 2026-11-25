-- Empujón: una sola perilla para que suene más fuerte y lleno, con cuatro maneras de hacerlo.
return {
  nombre = { es = "Maximizador", en = "Maximizer" }, tipo = "efecto", grupo = { es = "Dinámica", en = "Dynamics" },
  controles = {
    { "cuanto", { es = "Cuánto", en = "Amount" }, 0, 1, 0.4 },
    { "modo", { es = "Modo", en = "Mode" }, 0, 3, 0 },
  },
  presets = {
    { nombre = { es = "Cálido", en = "Warm" }, modo = 0, cuanto = 0.45 },
    { nombre = { es = "Con pegada", en = "Punchy" }, modo = 1, cuanto = 0.55 },
    { nombre = { es = "Brillante", en = "Bright" }, modo = 2, cuanto = 0.4 },
    { nombre = { es = "A tope", en = "Flat out" }, modo = 3, cuanto = 0.8 },
  },
  sonido = function(e, c)
    local function es(k) return mayor(c.modo, k - 0.5) * mayor(k + 0.5, c.modo) end
    -- Cuánto empuja cada banda en cada modo: graves, medios y agudos.
    local g = es(0) * 1.25 + es(1) * 1.4 + es(2) * 0.9 + es(3) * 1.4
    local m = es(0) * 1.0 + es(1) * 0.8 + es(2) * 1.0 + es(3) * 1.4
    local a = es(0) * 0.8 + es(1) * 1.0 + es(2) * 1.6 + es(3) * 1.3
    local function aplasta(x, k)
      local nivel = seguidor(abs(x), 0.006, 0.12)
      return x * maximo(nivel / db(-24), 0.02) ^ (0 - 0.55 * c.cuanto) * (1 + (k - 1) * c.cuanto)
    end
    local function f(x)
      local bajo = pasabajos(x, 180, 0)
      local alto = pasaaltos(x, 3500, 0)
      local y = aplasta(bajo, g) + aplasta(x - bajo - alto, m) + aplasta(alto, a)
      return curva(y, function(v) return math.tanh(v) end, "suave")
    end
    return f(e.izq), f(e.der)
  end,
  interfaz = {
    ancho = 300, alto = 280,
    dibujar = function(d, c, m)
      local colores = { "#ff9a52", "#ff5f6d", "#7fd0ff", "#c58cff" }
      local nombres = m.idioma == "en" and { "Warm", "Punch", "Bright", "Max" } or { "Cálido", "Pegada", "Brillo", "Tope" }
      local modo = math.floor(c.modo + 0.5)
      local col = colores[modo + 1]
      d.acero(0, 0, 300, 280, "#55405e", 10)
      d.grabado(150, 24, m.titulo, 22, col, "centro")
      -- El aro crece con lo que suena.
      for i = 0, 10 do
        local a = math.pi * (0.75 + 1.5 * i / 10)
        d.led(150 + math.cos(a) * 82, 128 + math.sin(a) * 82, 3, col, m.nivel * 10 > i and 1 or 0.06)
      end
      d.perilla(150, 128, 68, "cuanto", col, "#8d8f96")
      d.grabado(150, 219, math.floor(c.cuanto * 100 + 0.5) .. " %", 16, "#f2ecf5", "centro")
      d.lista(14, 238, 272, 28, "modo", nombres, col)
      d.led(30, 26, 5, col, math.min(1, m.nivel * 2))
      d.led(270, 26, 5, col, math.min(1, m.nivel * 2))
    end,
  },
}
