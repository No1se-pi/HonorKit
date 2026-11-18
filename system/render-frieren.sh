#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
# Compose static artwork for GRUB's built-in text menu.
set -eu

assets="$(dirname -- "$0")/frieren"
output=${1:?Supply the output PNG path}
fonts=/usr/share/fonts/truetype/dejavu

# The native list and its help text occupy the top left and the bottom.
# Keep the center left of the original artwork clear.
magick \
    \( "$assets/background.png" -resize '1560x1040^' -gravity center -extent 1560x1040 \) \
    \( -size 1560x420 gradient:'rgba(6,12,28,0.90)-rgba(6,12,28,0)' \) \
    -gravity north -compose over -composite \
    \( -size 1560x530 gradient:'rgba(6,12,28,0)-rgba(6,12,28,0.84)' \) \
    -gravity south -composite \
    \( "$assets/panel.png" -resize '410x171!' \) \
    -gravity northwest -geometry +1068+412 -composite \
    \( "$assets/seal.png" -resize 55x55 \) \
    -geometry +1245+427 -composite \
    -gravity northwest -fill '#fff2d7' -font "$fonts/DejaVuSerif-Bold.ttf" \
    -pointsize 33 -annotate +1192+501 'FRIEREN' \
    -stroke '#a48a64' -strokewidth 1 -draw 'line 1132,531 1414,531' \
    -stroke none -fill '#e0cab0' -font "$fonts/DejaVuSans.ttf" \
    -pointsize 17 -annotate +1179+552 'The Grimoire of Paths' \
    -strip -alpha off -depth 8 -interlace none "PNG24:$output"
