#!/usr/bin/env bash
# Sourced by `build-gradient-fills.sh` / `build-outer-shadow.sh` /
# `build-shape-flips.sh` to assert the local-only EPPlus project is in
# place before invoking it.
#
# The C# project at `tests/fixtures/shapes/dotnet-builder/` is
# gitignored \u2014 we ship the .xlsx + .hsx.png + .ours.png artefacts but
# not the EPPlus authoring code. To rebuild these fixtures locally,
# set the project up by hand (one-time):
#
#   mkdir -p tests/fixtures/shapes/dotnet-builder
#   cd tests/fixtures/shapes/dotnet-builder
#   dotnet new console -n FixtureBuilder
#   cd FixtureBuilder
#   dotnet add package EPPlus --version 7.*
#
# Then drop `Program.cs` (see the canonical version kept offline) and
# `strip-boms.py` into place. The fixture .xlsx files themselves are
# committed, so a fresh clone never NEEDS to run this.

require_dotnet_builder() {
  local here="$1"
  local proj="$here/dotnet-builder/FixtureBuilder"
  if [ ! -f "$proj/Program.cs" ] || [ ! -f "$proj/FixtureBuilder.csproj" ]; then
    cat >&2 <<EOF
error: tests/fixtures/shapes/dotnet-builder/FixtureBuilder is missing.

This fixture is authored offline via EPPlus. The committed .xlsx +
.hsx.png + .ours.png cover the visual diff for CI; to RE-AUTHOR the
.xlsx locally you need to set up the C# project. See the inline
instructions at the top of \`_dotnet-builder-guard.sh\`.
EOF
    return 1
  fi
  if ! command -v dotnet >/dev/null 2>&1; then
    echo "error: \`dotnet\` not in PATH (needed for EPPlus fixture build)." >&2
    return 1
  fi
  if [ ! -f "$here/dotnet-builder/strip-boms.py" ]; then
    echo "error: missing tests/fixtures/shapes/dotnet-builder/strip-boms.py" >&2
    return 1
  fi
  return 0
}
