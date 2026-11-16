#!/usr/bin/env bash
# Gate: every widget the engine can emit is documented, and the retired
# concatenated spellings do not creep back into the docs or the dispatcher.
set -euo pipefail

root="${1:-.}"
widgets="$root/tests/widget-names.txt"
docs="$root/docs/widgets.md"
control="$root/frontend/packages/ui/src/lib/components/Control.svelte"

test -f "$widgets"
test -f "$docs"
test -f "$control"

while IFS= read -r name; do
  [ -z "$name" ] && continue
  grep -Fq "\`$name\`" "$docs" || {
    echo "docs/widgets.md does not mention widget: $name" >&2
    exit 1
  }
done < "$widgets"

retired=(
  tableattrset
  tablelist
  icontheme
  cursortheme
  horizontalsubmodule
  gridconfiguration
)
for name in "${retired[@]}"; do
  if grep -Fqw "$name" "$docs" "$control"; then
    echo "retired concatenated widget name is still present: $name" >&2
    exit 1
  fi
done

grep -Fq 'table-attrset' "$control"
grep -Fq 'icon-theme' "$control"
grep -Fq 'nixos-modules' "$control"
grep -Fq '"widget": record.spec.widget.map(crate::render::widget_name)' \
  "$root/engine/src/api/json.rs"

echo "parity widget names are documented and kebab-cased"
