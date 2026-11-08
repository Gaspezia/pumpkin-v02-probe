#!/bin/bash
# Downloads Velocity 4.3.0 build 44 and ViaVersion/ViaBackwards 5.12.2 into the given dir, sha256-checked.
set -euo pipefail
D=${1:?dir}; mkdir -p "$D/plugins"
get() { # url sha256 dest
  [ -f "$3" ] && echo "$2  $3" | sha256sum -c --quiet - 2>/dev/null && return 0
  curl -fsSL -o "$3" "$1"; echo "$2  $3" | sha256sum -c --quiet -
}
get https://fill-data.papermc.io/v1/objects/85be73b90f295acd8d5d45a00ad10bac791d4b72b7554172612a89565ebaaa8f/velocity-4.3.0-44.jar 85be73b90f295acd8d5d45a00ad10bac791d4b72b7554172612a89565ebaaa8f "$D/velocity.jar"
get https://repo.viaversion.com/com/viaversion/viaversion/5.12.2/viaversion-5.12.2.jar 24e4876fefe73e08a4c87cd6c457ee0916e62dd69656631caf41e26f909d504b "$D/plugins/ViaVersion-5.12.2.jar"
get https://repo.viaversion.com/com/viaversion/viabackwards/5.12.2/viabackwards-5.12.2.jar 9a148a80ea22074f4bd1626cdbe254a4cd998bf44e7ceaff4e285a1b6b35da42 "$D/plugins/ViaBackwards-5.12.2.jar"
