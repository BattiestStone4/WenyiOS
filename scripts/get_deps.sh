#!/bin/bash

AX_ROOT=.arceos

test ! -d "$AX_ROOT" && echo "Cloning repositories ..." || true
test ! -d "$AX_ROOT" && git clone https://github.com/battieststone4/arceos -b all_final "$AX_ROOT" --depth=1 || true

$(dirname $0)/set_ax_root.sh $AX_ROOT
