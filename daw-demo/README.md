# Beethoven DAW Demo

Estudio de música propio de Beethoven Craft, en demostración; es el DAW que abre el menú de Beethoven Craft. Español e inglés, según el sistema o lo que se elija en **Archivo › Idioma**. *(English below.)*

## Qué hace

- **La Playlist (F5) es el fondo** y ocupa toda la pantalla; el **Mixer** (F9) es otra pestaña, que se puede sacar como ventana. **Ritmo** (F6), **Piano roll** (F7) e **Instrumento** (F8) son ventanas encima, en el menú **Ventana**, que también se pueden fijar como pestaña. Todo con el metal y el vidrio de Movie. Abajo siempre dice qué hace el ratón en ese lugar.
- **Pistas de audio**: arrastra o pega (⌘V) WAV, MP3, OGG, Opus, FLAC, M4A, AIFF, CAF o un video. Cada audio toma su propia pista del Mixer. Soltado en la Playlist cae donde está el ratón; en una tira del Mixer, va a esa pista; en Ritmo o Instrumento, se vuelve instrumento. Se mueven, recortan, cortan y ajustan (volumen, lado, fundidos, velocidad). **Graba** el micrófono.
- **Mixer** de 12 pistas, 4 envíos y general: ranuras de efectos que abren su ventana con un clic, cuánto efecto por ranura, ecualizador de tres bandas por pista, ancho, polaridad, mudo, solo, medidores con aviso de saturación. 25 efectos: ecualizador con curva y analizador, compresor con su curva, limitador, compuerta, multibanda, quita eses, transitorios, reverb, ecos, coro, flanger, faser, trémolo, cinta, lo-fi y más.
- **Línea de tiempo suave**: los clips siguen al ratón y caen en su lugar; imán, tijera (⌘3), pincel (⌘2), borrador (⌘4), elegir varios, copiar, pegar, duplicar (⌘D), partir en el cabezal (⌘E), recortar de los dos lados, fundidos en las esquinas, mapa abajo.
- **Elegir instrumento** al crear un patrón o una pista, y cambiarlo desde el Piano roll.
- **Asistente de IA** (F10): le dices qué quieres y edita el proyecto. Encuentra solo lo que tengas (Claude Code, Ollama, LM Studio, llama.cpp, o claves en el entorno); también acepta tu clave o cualquier servidor con su dirección y puerto. Mide el audio y la mezcla (niveles, silencios, tempo, tonalidad), maneja plugins, exporta y sabe qué tienes seleccionado. Todo lo que hace se puede deshacer. El mismo asistente está en Movie.
- **Instrumentos grabados**: unos 195, nota por nota, del banco **MuseScore General** (MIT): pianos, guitarras, bajos, cuerdas y secciones de orquesta, metales, maderas, coros, sintetizadores, instrumentos del mundo, baterías y percusión. Van por familia en la izquierda. El banco (40 MB) va dentro de la app; al compilar lo baja `empaque/sonidos.sh` a `sonidos/`.
- **Crear en Lua**: tus propios instrumentos y efectos, con editor dentro de la app; también herramientas de notas. Los efectos de fábrica son de Lua.
- **Plugins VST3 y CLAP**, con su ventana (Mac) o sus controles.
- **SoundFont (.sf2 y .sf3), SFZ y audio suelto** como instrumentos. **Teclados MIDI** y anotar lo que tocas.
- **Ritmos listos**, reparto parejo, metrónomo, swing, escalas.
- **Abre `.flp`**: tempo, canales, notas, patrones y clips de patrón. **MIDI** de entrada y salida.
- **Exporta** WAV y pistas separadas; MP3, FLAC, OGG y M4A si ffmpeg está instalado.

Lo que todavía no: automatización, compensación de latencia de los plugins, estirar audio sin cambiar el tono, VST2 y Audio Units, SFZ con grabaciones comprimidas raras. Los plugins internos de FL Studio son cerrados y solo abren dentro de FL.

## Compilar y probar

    cargo run --release
    cargo test --release
    cargo run --release --example plugins     # prueba los plugins instalados
    cargo run --release -- --medir            # velocidad del motor
    empaque/empacar.sh                        # arma la app de Mac

## Hacer un instrumento

Los de fábrica son grabaciones; en **Crear en Lua** (a la izquierda) escribes los tuyos.

Un guion arma el sonido **una vez**; después lo toca el motor, sin Lua en el hilo de audio. Se guardan en `Music/Beethoven DAW Demo/Lua`, cada uno en su carpeta (ver abajo).

```lua
return {
  nombre = { es = "Mi bajo", en = "My bass" },   -- o solo "Mi bajo"
  tipo = "instrumento",                           -- instrumento, efecto o herramienta
  grupo = "Míos",
  voces = 8,
  controles = { { "corte", "Corte", 100, 8000, 900 } },  -- clave, nombre, mínimo, máximo, valor
  sonido = function(n, c)                         -- n.hz, n.fuerza, n.azar
    local x = sierra(n.hz) + pulso(n.hz / 2, 0.5)
    return pasabajos(x, c.corte, 0.3) * adsr(0.01, 0.2, 0.6, 0.1) * n.fuerza * 0.4
  end,                                            -- una señal, o dos (izquierda, derecha)
}
```

| Función | Qué da |
| --- | --- |
| `seno(hz, fase)` `sierra(hz, fase)` `triangulo(hz, fase)` `pulso(hz, ancho)` `ruido()` | osciladores (la fase va en vueltas: sirve para FM) |
| `adsr(ataque, caída, sostén, soltar)` `golpe(segundos)` | envolventes |
| `pasabajos(x, hz, filo)` `pasaaltos` `pasabanda` | filtros |
| `graves(x, hz, dB, ancho)` `campana` `agudos` `muesca(x, hz, ancho)` | bandas de ecualizador |
| `seguidor(x, ataque, soltar)` `db(x)` | nivel y decibeles (compresores, compuertas) |
| `escalon(x, n)` `retener(x, hz)` `pasatodo(x, hz)` | triturar, muestrear, mover la fase |
| `retardo(x, seg, repite, tope)` `sala(x, tamaño, brillo)` `cuerda(x, hz, caída, brillo)` | espacio y cuerda |
| `muestra(velocidad, "izq"/"der")` | la grabación del canal (WAV, SF2, SFZ) |
| `tanh` `abs` `sen` `minimo` `maximo` `mezclar(a, b, k)` y `+ - * / ^` | cuentas |

En un **efecto**, `sonido(e, c)` recibe `e.izq` y `e.der`. Una **herramienta** tiene `notas(lista, c)`, con `paso`, `largo`, `tono` y `fuerza` de cada nota, y devuelve la lista nueva.

### Con su propia interfaz

Un instrumento o efecto puede dibujar su ventana y elegir su estilo. `dibujar` corre en cada cuadro; mira `efectos/vozencaja/vozencaja.lua` («Voz en caja»).

```lua
interfaz = {
  ancho = 300, alto = 160,
  dibujar = function(d, c, m)   -- c: valores de los controles · m: tiempo, nivel, tocando, idioma, x, y
    d.degradado(0, 0, 300, 160, "#2a2018", "#0c0a08", 8)
    d.texto(150, 20, "MI EFECTO", 20, "#ffd95e", "centro")
    d.perilla(80, 90, 30, "corte", "#ff6a3c")
    d.circulo(220, 90, 20 + m.nivel * 30, "#ffd95e")
  end,
}
```

| Pinta | Se mueve |
| --- | --- |
| `caja(x,y,w,h,color,r)` `degradado(x,y,w,h,arriba,abajo,r)` `borde(x,y,w,h,color,grueso,r)` `circulo(x,y,r,color)` `aro(x,y,r,color,grueso)` `linea({x,y,…},color,grueso)` `poligono({x,y,…},color)` `texto(x,y,s,tamaño,color,"centro"/"derecha")` `metal(x,y,w,h,r)` `hueco(…)` `vidrio(x,y,w,h,color,r)` `tornillo(x,y,r)` | `perilla(x,y,r,clave,color)` `tirador(x,y,w,h,clave,color)` `boton(x,y,w,h,texto,clave,color)` `pad(x,y,w,h,claveX,claveY)` |

`imagen(x,y,w,h,"fondo.png",opacidad)` pinta un PNG de la carpeta del guion.

### Su carpeta: imágenes, logo y otros Lua

Cada guion vive en una carpeta con su nombre, por ejemplo `Lua/voz gruesa/`:

```
voz gruesa/
  voz gruesa.lua    -- el principal (o el único .lua)
  ayuda.lua         -- se usa con incluir("ayuda")
  logo.png          -- logo = "logo.png" en el guion
  fondo.png
  fondos/noche.png  -- también una subcarpeta
```

- `logo = "logo.png"` lo muestra en la lista, en el menú de efectos y en su ventana.
- `local ayuda = incluir("ayuda")` corre otro `.lua` de la misma carpeta y devuelve lo que regrese.
- Solo PNG, hasta 2048 px por lado, 8 MB cada uno y 32 MB por carpeta. El guion solo ve los nombres de su carpeta.
- En el editor, **Abrir su carpeta** la muestra para que pongas ahí tus imágenes. Si editas uno de fábrica, tu copia se lleva sus imágenes.
- Mira `efectos/vozgruesa/` («Voz gruesa»). Los `.lua` sueltos de antes siguen sirviendo.

Los guiones no pueden abrir archivos ni usar el sistema, y tienen tope de memoria y de pasos. Los plugins VST3 y CLAP sí son programas completos: instala solo los de confianza.

## El asistente y tus datos

- A la IA se le mandan los nombres, notas y ajustes del proyecto; nunca el audio.
- Con **Claude Code** se usa el que tienes instalado, con tu cuenta, y sin sus herramientas de archivos ni de terminal: solo puede usar las del proyecto.
- Las claves se guardan en `Music/Beethoven DAW Demo/asistente.json`, que solo tú puedes leer, y solo se mandan a la dirección que pusiste (con `https`, o en tu propia máquina).
- El servidor **MCP** (para que otro programa controle este) solo escucha en tu máquina y pide una llave.

## English

Beethoven Craft's own music studio, as a demo. Arrangement, Rhythm, Notes and Mixer; instruments, effects and note tools written in **Lua**; **VST3 and CLAP** plugins; **SoundFont, SFZ and WAV** instruments; opens **.flp** (tempo, channels, notes, patterns and pattern clips) and **MIDI**; exports WAV and MIDI. An **AI assistant** (F10) edits the project for you with Claude Code, Ollama, LM Studio, an API key or any compatible server. Lua scripts can draw their own interface; each one lives in its own folder with its PNG images, a logo and helper Lua files (`incluir`). It cannot load FL Studio's internal plugins, VST2 or Audio Units. The language follows the system, or **File › Language**. Function names in Lua are in Spanish; see the table above.

## Licencia · License

MIT o / or Apache-2.0. Escrito desde cero, sin código de Lemon Studio / written from scratch, without Lemon Studio code: [AVISOS.md](AVISOS.md).
