package rewrite

ir_version: "0.2"
systems: {s: {}}
machines: {m: {system: "s", start: "a", attack: {enabled: true}, states: {a: {kind: "benign", attack_label: {technique: "T1110"}, terminal: true}}}}
instance_groups: {g: {machine: "m", count: 1}}
time_profiles: {tp: {start: "2026-01-01T00:00:00Z"}}
scenario: {id: "bad_benign_label", systems: ["s"], instance_groups: ["g"], time_profile: "tp"}
