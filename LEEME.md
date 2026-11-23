# Beethoven Craft

Creado por mixter-ha99.

Compositor de video y audio. App con sus propias ventanas, en español.

Al abrirla sale el menú: proyectos recientes, **Nuevo video** (abre **Beethoven Movie**, el editor de video) y **Nueva canción** (abre **Beethoven DAW**, el estudio de música: rack, piano roll, mezcla y plugins VST3/CLAP, en `daw/`). Lo de abajo es Beethoven Movie.

Pestañas como de navegador: se cierran, se ordenan arrastrando, `+` abre otra y con clic derecho se sacan a su propia ventana.

- **EFECTOS**: catálogo con muestras (estilos listos, video, transiciones, audio, títulos), vista previa y los controles de lo elegido.
- **VIDEO**: origen, controles y color a la izquierda; programa al centro; biblioteca a la derecha; herramientas, pistas y medidor abajo.
- **AUDIO**: consola, rack, piano roll y editor, con las pistas de sonido abajo.
- **PLAYLIST**: el resultado final con todas las pistas.

Herramientas: V selección, A hacia adelante, B ondulado, N rodar, X estirar velocidad, C cuchilla (Shift: todas), Y deslizar, U correr, H mano, Z lupa, T texto.

Teclas: espacio / J K L, I y O marcan tramo, `;` levanta, `'` extrae, Q y W recortan al cursor, M marca, ↑ ↓ saltan cortes, ← → un cuadro (Shift: 5), S o ⌘K dividir, ⌘D disolvencia, ⌫ borrar, ⇧⌫ borrar y cerrar, ⌘M exportar, `\` ver todo, R grabar.

Imagen y sonido de un video quedan vinculados; clic derecho para desvincular. Automatización: clic derecho en la cabecera de la pista.

En Rack y Piano roll, la columna de la izquierda elige el instrumento; «Suena» le cambia el sonido sin tocar las notas.

El proyecto `.bcraft` es un binario propio, comprimido y con suma de control. «Guardar con todo incluido» mete también los videos y audios. Los proyectos viejos (JSON) se siguen abriendo.

Funciona en Mac, Windows y Linux. Necesita `ffmpeg`: en Mac el de Homebrew; en Windows y Linux, junto a la app o en el PATH. Grabaciones, voces separadas y autoguardado van en la carpeta de videos del usuario (`~/Movies`, `Videos` o `~/Videos`), dentro de `Beethoven Craft`.

    empaque/empacar.sh        # Mac: construir la app (también compila daw/)
    empaque/empacar-linux.sh  # Linux: beethoven-craft-linux.tar.gz (necesita libasound2-dev y pkg-config)
    powershell -ExecutionPolicy Bypass -File empaque\empacar-windows.ps1 [-Ffmpeg C:\ffmpeg\bin]   # Windows: zip
    (cd daw && cargo test --release -p os-app)   # pruebas de Beethoven DAW
    cargo test --release      # pruebas
    VIDEO=/ruta/video.mp4 cargo test --release -- --ignored --test-threads 1

Licencia GPL-3.0 o posterior. Créditos en `AVISOS.md`.
