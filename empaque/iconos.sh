#!/bin/zsh
# Rehace los íconos nítidos desde recursos/beethoven.jpg (cada tamaño por separado).
set -e
cd "$(dirname "$0")/.."
T=$(mktemp -d); mkdir $T/i.iconset $T/d.iconset
hacer() { # nombre tamaño
  local s=$2 c=$(( $2 * 824 / 1024 )) r=$(( $2 * 185 / 1024 )) src="recursos/beethoven.jpg" crop=""
  # En tamaños chicos, la cara más de cerca; si no, el grabado se empasta.
  [ $s -le 64 ] && crop="-crop 700x700+130+90 +repage"
  magick $src ${=crop} -colorspace RGB -filter Lanczos -resize ${c}x${c}! -colorspace sRGB -unsharp 0x0.6+0.9+0.01 -alpha set $T/c.png
  magick -size $((c*4))x$((c*4)) xc:none -fill white -draw "roundrectangle 0,0,$((c*4-1)),$((c*4-1)),$((r*4)),$((r*4))" -filter Box -resize ${c}x${c} $T/m.png
  magick $T/c.png $T/m.png -compose DstIn -composite -background none -gravity center -compose Over -extent ${s}x${s} $T/i.iconset/$1.png
  magick -size 1024x1024 xc:none -fill "#f4efe4" -stroke "#6b655b" -strokewidth 10 -draw "polygon 180,50 650,50 844,244 844,974 180,974" -fill "#d9d2c3" -draw "polygon 650,50 650,244 844,244" -filter Lanczos -resize ${s}x${s} $T/h.png
  local f=$(( s * 560 / 1024 ))
  magick $src ${=crop} -colorspace RGB -filter Lanczos -resize ${f}x${f}! -colorspace sRGB -unsharp 0x0.6+0.9+0.01 $T/f.png
  magick $T/h.png $T/f.png -gravity center -geometry +0+$(( s * 70 / 1024 )) -composite $T/d.iconset/$1.png
}
for s in 16 32 128 256 512; do hacer icon_${s}x$s $s; hacer icon_${s}x$s@2x $((s*2)); done
iconutil -c icns $T/i.iconset -o empaque/icono.icns
iconutil -c icns $T/d.iconset -o empaque/proyecto.icns
cp $T/i.iconset/icon_256x256@2x.png recursos/icono512.png
