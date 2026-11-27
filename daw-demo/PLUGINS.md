# Hacer plugins para Beethoven DAW

*(English summary at the end.)*

Un plugin es **una carpeta con un guion de Lua**. Quien lo recibe la suelta en el programa y ya funciona: no hay que compilar nada, ni reiniciar, ni tocar el código del programa. Los efectos y sintetizadores de fábrica están hechos con esto mismo, sin privilegios: lo que hace uno de fábrica lo puede hacer el tuyo.

¿Programas en C++ o Rust? El DAW también abre **VST3 y CLAP**.

## Empezar en un minuto

1. En la biblioteca (izquierda), **Tus plugins › + Efecto en Lua** o **+ Instrumento en Lua**. Se abre una plantilla completa, con su ventana.
2. Cámbiala y aprieta **Guardar y oír**.
3. ¿Prefieres tu editor? **Abrir la carpeta**: cada vez que guardes, el programa lo carga solo. Si tiene un error te lo dice y sigue sonando la versión anterior.

## Compartirlo

Clic derecho en tu plugin › **Empacar para compartir…** hace un `.zip`. Quien lo reciba lo suelta en la ventana (o **Instalar plugin…**). También sirve soltar la carpeta o un `.lua` suelto. Un proyecto que use un plugin que no está lo dice al abrirse, y suena en cuanto lo instalas.

## La carpeta

```
mi plugin/
  mi plugin.lua     el principal: se llama como la carpeta, o plugin.lua, o es el único
  ayuda.lua         otros guiones: incluir("ayuda")
  logo.png          imágenes PNG
  sonidos/do.wav    audios WAV, OGG, FLAC o MP3
```

Hasta una subcarpeta, 256 archivos y 160 MB.

## El guion

```lua
return {
  nombre = { es = "Mi efecto", en = "My effect" },   -- que diga lo que hace
  tipo = "efecto",            -- efecto, instrumento o herramienta (de notas)
  grupo = { es = "Míos", en = "Mine" },
  autor = "Tu nombre", version = "1.0", api = 1,
  descripcion = { es = "…", en = "…" },
  logo = "logo.png",
  controles = { { "cuanto", { es = "Cuánto", en = "Amount" }, 0, 1, 0.5 } },
  presets = { { nombre = { es = "Suave", en = "Soft" }, cuanto = 0.2 } },
  sonido = function(e, c) return e.izq * c.cuanto, e.der * c.cuanto end,
  interfaz = { ancho = 420, alto = 260, dibujar = function(d, c, m) … end },
}
```

`sonido` corre **una vez**: arma un grafo que después toca el motor, sin Lua en el hilo de audio. Por eso un guion lento no corta el sonido. Las funciones de sonido están en el [README](README.md#hacer-un-instrumento).

### En vivo: Lua mientras suena

Para lo que un grafo no puede, `vivo` corre en el hilo de audio, muestra por muestra o por bloques. Sirve solo o después de `sonido`.

```lua
vivo = function(v)              -- corre una vez: aquí van tus tablas y variables. v.sr, v.datos
  local z = 0.0
  return function(l, r, c, m)   -- en cada muestra; c son los controles
    z = z + (l - z) * 0.1
    return z, z
  end
end
```

En vez de la función puede devolver una tabla:

| Campo | Qué es |
| --- | --- |
| `muestra(l, r, c, m)` o `bloque(izq, der, n, c, m)` | el sonido; `bloque` cambia las tablas en su lugar |
| `nota(tono, fuerza, c)` `suelta(tono)` | lo que se toca (instrumentos, y `tipo = "notas"`) |
| `rueda(x)` `mando(n, x)` | la rueda de tono (-1 a 1) y los mandos MIDI (0 a 1) |
| `latencia` | muestras que se tarda; el Mixer empareja las demás pistas |

A la mano: `fft(re, im)` y `fft(re, im, true)` (hasta 4096), `espectral(n, function(re, im, lado, c, m) … end)` (por frecuencias, ya con sus ventanas), `retraso(n)` (`pon(x)`, `lee(d)`), `tanh`, `db`, `hz(tono)`, `mide("nombre", v)` (con `medidas = { "nombre" }` en la ficha), `toca(tono, fuerza, segundos)` y `graba(true)`. En `m`: `pos` (negras), `bpm`, `tocando`, `otra_izq`, `otra_der`.

- Con `tipo = "notas"` el guion cambia las notas antes del instrumento: mira `instrumentos/arpegiador.lua`.
- `d.campo(x, y, w, "clave")` pone un renglón para escribir; llega como `v.datos.clave`.
- No crees tablas ni textos al sonar: hazlo antes. Si el guion se cuelga o da infinito, se apaga, avisa y el sonido pasa igual.

### Instrumentos con audios

| En la ficha | Con qué suena `muestra()` |
| --- | --- |
| `muestras = { { archivo = "sonidos/do.wav", nota = 60, desde = 48, hasta = 72, fuerza_min = 0, fuerza_max = 127 }, … }` | los audios que trae el plugin |
| *(nada)* | el audio que elija quien lo usa (`d.archivo` pone el botón) |
| `muestras = "pads"` | 16 audios, uno por tecla desde la 36 (`d.pads`; controles `v1`…`v16` volumen, `t1`… tono, `l1`… lado, `g1`… grupo de corte) |
| `muestras = "cortes"` | el audio partido en pedazos desde la 48 (controles `partes`, `donde`, `fino`; `m.cortes`; `d.escribir`) |

### Efectos especiales

| Si tu guion usa… | pasa esto |
| --- | --- |
| `convolucion(x, "izq")` | necesita una huella: la de `huella = "sala.wav"` en la ficha, la que cargue quien lo usa, o uno de los lugares de fábrica (control `espacio`, 0 a 7). También cuentan `largo` y `reves` |
| `e.otra_izq`, `e.otra_der` | oyen la pista que diga el control `pista` (`d.pista` pone la lista) |
| `mide("nombre", x)` | `m.medidas.nombre` en la ventana |
| `mide("control", x)` | se vuelve un controlador: aparece en «Moverlo con…» de todas las perillas |

### La ventana

`local equipo = incluir("equipo")` trae ventanas de aparato completas: `equipo.rack{…}`, `equipo.pedal{…}`, `equipo.cinta{…}` y `equipo.bulbos{…}`. Para dibujarla tú, `dibujar(d, c, m)` corre en cada cuadro:

- **Piezas de metal:** `acero` `bandeja` `hueco` `grabado` `led` `aguja` `palanca` `pisoton` `lista` `rota` `valvula` `carrete` `asa`
- **Dibujo:** `caja` `degradado` `borde` `circulo` `aro` `linea` `poligono` `texto` `imagen` `vidrio` `tornillo`
- **Controles:** `perilla` `tirador` `boton` `pad` y `poner(clave, valor)`
- **Que hacen cosas:** `prender` (bypass), `grafica(x,y,w,h,"niveles"/"agudos"/"frecuencias"/"cola"/"movimiento"/"onda")`, `onda(x,y,w,h,color)` (lo que entra y sale en este momento), `historia(x,y,w,h,color)` (los últimos segundos), `ecualizador` (controles `h1`,`g1`,`q1`,`t1`…, `corta`, `tope`), `teclado`, `archivo`, `pads`, `pista`, `escribir`
- **`m`:** `titulo` `nombre` `nombres[clave]` `idioma` `tiempo` `nivel` `tocando` `activo` `onda` `onda_entra` `espectro` `espectro_entra` `historia` `historia_entra` `nivel_entra` `medidas` `huella` `cortes` `pad` `archivo` `x` `y` `clic` `apretado`

## En otros programas

`plugins/empacar.sh` (Mac y Linux) y `plugins/empacar.ps1` (Windows) arman **Beethoven.vst3** y **Beethoven.clap**: cada archivo trae todos los efectos e instrumentos, que aparecen como «Beethoven …», y también los que tengas instalados en tu carpeta de plugins. Los controladores, las herramientas y los de notas solo viven en el DAW.

| | VST3 | CLAP |
| --- | --- | --- |
| Mac | `~/Library/Audio/Plug-Ins/VST3` | `~/Library/Audio/Plug-Ins/CLAP` |
| Windows | `C:\Program Files\Common Files\VST3` | `C:\Program Files\Common Files\CLAP` |
| Linux | `~/.vst3` | `~/.clap` |

## Qué revisa el programa al instalar

Que arme, que no suene infinito ni a más de 8 veces el tope, que no pase del 70 % de un procesador con una voz, que su ventana dibuje y que no le falten audios. **Revisar**, en el editor, hace lo mismo. Si algo falla no se instala nada.

Los guiones **no pueden** abrir archivos, usar el sistema ni la red, y tienen tope de memoria y de pasos. Un plugin ajeno no reemplaza a uno de fábrica. Lo que se quita o se actualiza queda en la carpeta `Quitados`.

`api = 1` es la versión de lo que puede usar un guion. Lo que ya existe no cambia de nombre; si un plugin pide un número mayor, el programa avisa que falta actualizarlo.

## English

A plugin is a folder with a Lua script (plus PNG images and audio files). Drop the folder, a `.zip` or a single `.lua` on the window and it works: no build step, no restart. Saved edits reload live; if a script has an error the previous version keeps playing. Factory effects and synths use this same API with no special access. Start from **Your plugins › + Effect in Lua**, share with **Pack to share…**. Scripts are sandboxed (no files, system or network). Function names are in Spanish; see the tables above. Native developers can use VST3 or CLAP.
