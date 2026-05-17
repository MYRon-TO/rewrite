package rewrite

ir_version: "0.1"
systems: {s: {}}
machines: {m: {system: "s", start: "a", states: {a: {terminal: true}}}}
instance_groups: {g: {machine: "m", count: 1}}
time_profiles: {tp: {start: "2026-01-01T00:00:00Z"}}
