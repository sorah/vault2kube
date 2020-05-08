#!/bin/bash
cd "$(dirname "$0")"
(
  for x in setup_*.jsonnet; do
    jsonnet $x | ruby -ryaml -rjson -e 'puts JSON.parse(ARGF.read).to_yaml'
  done
) > ../setup.yml

for x in example_*.jsonnet; do
  jsonnet $x | ruby -ryaml -rjson -e 'puts JSON.parse(ARGF.read).to_yaml' > ../${x%.jsonnet}.yml
done

