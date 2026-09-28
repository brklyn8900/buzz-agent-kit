#!/usr/bin/env python3
"""Packaging checks; actual agent behavior is verified in fresh-host acceptance."""
import json
import pathlib
import re
root = pathlib.Path(__file__).resolve().parents[1]
for name in ('setup', 'room'):
    text = (root / 'skills' / name / 'SKILL.md').read_text()
    head = text.split('---', 2)[1]
    fields = dict(line.split(': ', 1) for line in head.strip().splitlines())
    assert fields['name'] == name and re.fullmatch('[a-z0-9-]{1,64}', name)
    assert 0 < len(fields['description']) <= 1024
    assert '${PLUGIN_ROOT}' not in text and '${CLAUDE_PLUGIN_ROOT}' not in text
for path in ('.claude-plugin/plugin.json', 'plugin.json'):
    data = json.loads((root / path).read_text())
    assert not {'hooks', 'mcpServers'} & data.keys()
assert 'Apache License' in (root / 'LICENSE').read_text()
for name in ('AGENTS.snippet.md', 'buzz.config.example.json'):
    assert (root / 'templates' / name).is_file()
print('PASS: shared skills, manifests, generic templates and license packaged')
