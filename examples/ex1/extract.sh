#!/bin/zsh

NAME=$0

TRAPZERR() {
    echo "$0: An error occurred: $?" >&2
    exit 1
}

set -e

# Path to gerbv
GERBV=${GERBV:-gerbv}

# Intended size of the extracted bitmaps, in inches
SIZE=${SIZE:-9x6}

# Bottom-left Gerber coordinates of extracted bitmaps, in inches
ORIGIN=${ORIGIN:-2x-7}

# IMPORTANT NOTE: You must manually determine the origin coordinates
# and enter them in the configuration file into the "origin" field.

# You can determine the coordinates by modifying gerbv, for example
# by adding the following to main.c in gerbv 2.9.6 around line 1048:

# printf("Render info: DPI %g %g origin %g %g width %g height %g\n",
#        renderInfo.scaleFactorX,
#        renderInfo.scaleFactorY,
#        renderInfo.lowerLeftX,
#        renderInfo.lowerLeftY,
#        renderInfo.displayWidth,
#        renderInfo.displayHeight);

# Rendering resolution
DPI=${DPI:-600}

# Path to directory containing gerbers, for 6 layers named F, In1,
# In2, In3, In4 and B
PROJ=${PROJ:-/path/to/your/project/base_name}

# Output path
OUT=${OUT:-data}

if which $GERBV >/dev/null 2>&1 ; then
    echo "$NAME: Will run gerbv from $GERBV"
else
    echo "$NAME: Cannot locate gerbv, is it installed?" >&2
    exit 1
fi

mkdir -p $OUT

n=1

run() {
    echo "$NAME: Running $*"
    $*
}

for x in F In1 In2 In3 In4 B ; do
    SRC_GBR=${PROJ}-${x}_Cu.gbr
    DST_GBR=$OUT/lay$n.gbr
    PNG=$OUT/lay$n.png
    (( n++ ))

    if [ ! -e $DST_GBR ] || [ $SRC_GBR -nt $DST_GBR ]; then
	run cp $SRC_GBR $DST_GBR
    else
	echo "$NAME: $DST_GBR not older than source"
    fi
    if [ ! -e $PNG ] || [ $DST_GBR -nt $PNG ]; then
	run $GERBV \
	    -O$ORIGIN \
	    -W$SIZE \
	    -D$DPI \
	    $DST_GBR \
	    --export=png \
	    --output $PNG 2>/dev/null
    else
	echo "$NAME: $PNG not older than source"
    fi
done

run cargo run --release --bin capest -- --config etc/capest.cfg
