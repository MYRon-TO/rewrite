package rewrite

ir_version: "0.2"
entity: {users: ["alice"]}
systems: {s: {}}
machines: {m: {system: "s", start: "a", states: {a: {terminal: true}}}}
instance_groups: {g: {machine: "m", count: 1}}
time_profiles: {tp: {start: "2026-01-01T00:00:00Z"}}
scenario: {id: "bad_entity", systems: ["s"], instance_groups: ["g"], time_profile: "tp"}
