#!/usr/bin/env nu

overlay use mod.nu
scope commands
| where name in [helper mod]
| select name
| to nuon --raw
