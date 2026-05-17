# IR 合约 v0.1

## 1. 结论

阶段 1 之后，Rust 运行时只读取 `cue export` 生成的规范化 JSON IR，不读取 CUE，不理解 CUE 的继承、合并和默认值逻辑。

IR v0.1 的固定原则：

1. 顶层结构固定。
2. 对象集合使用 map，key 为对象 ID。
3. 引用统一使用字符串 ID 或 `ctx.<scope>.<name>`。
4. 默认值应在导出的 JSON 中展开。
5. IR 不包含执行逻辑、脚本、跨系统因果、攻击链对象或攻击比例控制。

## 2. 顶层结构

规范化 JSON IR 顶层结构如下：

```json
{
  "ir_version": "0.1",
  "systems": {},
  "generators": {},
  "context": {},
  "templates": {},
  "machines": {},
  "instance_groups": {},
  "time_profiles": {},
  "scenario": {}
}
```

字段说明：

| 字段 | 类型 | 必填 | 说明 |
|---|---:|---:|---|
| `ir_version` | string | 是 | IR 合约版本，当前为 `0.1`。 |
| `systems` | map | 否 | system 定义集合。 |
| `generators` | map | 否 | generator 定义集合。 |
| `context` | object | 否 | 四层上下文定义。 |
| `templates` | map | 否 | template 定义集合。 |
| `machines` | map | 否 | machine 定义集合。 |
| `instance_groups` | map | 否 | instance_group 定义集合。 |
| `time_profiles` | map | 否 | time_profile 定义集合。 |
| `scenario` | object | 否 | 单个场景入口。 |

> 当前 MVP 约定一个 IR 文件只包含一个 `scenario`。

## 3. ID 与 map key 规则

所有核心对象 ID 使用字符串，格式：

```text
^[A-Za-z][A-Za-z0-9_-]*$
```

对象集合采用 map 形式，map key 必须等于对象内部 `id`：

```json
{
  "systems": {
    "linux_auth": {
      "id": "linux_auth"
    }
  }
}
```

不要在 IR 中混用 array 形式对象集合。下面这种不是 v0.1 合约：

```json
{
  "systems": [
    {"id": "linux_auth"}
  ]
}
```

## 4. 核心对象结构

### 4.1 `system`

```json
{
  "id": "linux_auth",
  "kind": "linux_auth",
  "description": "Linux auth log domain"
}
```

字段：

| 字段 | 类型 | 必填 | 默认值 | 说明 |
|---|---:|---:|---:|---|
| `id` | string | 是 | 无 | system ID。 |
| `kind` | string | 否 | 无 | 人类可读分类。 |
| `description` | string | 否 | 无 | 说明。 |

`system` 只表示命名空间和日志域，不定义实例数量、调度和跨系统关系。

### 4.2 `generator`

```json
{
  "id": "username",
  "type": "username",
  "params": {}
}
```

字段：

| 字段 | 类型 | 必填 | 默认值 | 说明 |
|---|---:|---:|---:|---|
| `id` | string | 是 | 无 | generator ID。 |
| `type` | string | 是 | 无 | 内置 generator 类型名。 |
| `params` | object | 是 | `{}` | JSON 兼容参数。 |
| `description` | string | 否 | 无 | 说明。 |

`generator` 只声明值来源契约，不包含代码、hook、插件或外部调用。

### 4.3 `context`

```json
{
  "global": {
    "tenant_id": {
      "source": {"const": "tenant-a"}
    }
  },
  "system": {
    "linux_auth": {
      "hostname": {
        "source": {"const": "host-1"}
      }
    }
  },
  "instance": {
    "ssh_login_flow": {
      "user": {
        "source": {"generator": "username"}
      }
    }
  },
  "event": {
    "attempt_id": {
      "source": {"generator": "uuid"}
    }
  }
}
```

四层作用域：

| 作用域 | JSON 字段 | 共享范围 |
|---|---|---|
| global | `context.global` | 整个 scenario。 |
| system | `context.system.<system_id>` | 单个 system。 |
| instance | `context.instance.<scope_id>` | 单个 machine instance。 |
| event | `context.event` | 单条事件。 |

上下文变量结构：

```json
{
  "source": {
    "const": "value"
  },
  "description": "optional"
}
```

`source` 支持三种形式：

```json
{"const": "fixed value"}
```

```json
{"generator": "username", "params": {}}
```

```json
{"ref": "ctx.global.tenant_id"}
```

### 4.4 `template`

```json
{
  "id": "ssh_login_success",
  "system": "linux_auth",
  "event_type": "ssh_login_success",
  "fields": {
    "user": {"ref": "ctx.instance.user"},
    "host": {"ref": "ctx.system.hostname"},
    "result": {"const": "success"}
  }
}
```

字段：

| 字段 | 类型 | 必填 | 默认值 | 说明 |
|---|---:|---:|---:|---|
| `id` | string | 是 | 无 | template ID。 |
| `system` | string | 是 | 无 | 所属 system ID。 |
| `event_type` | string | 否 | 无 | 事件类型名。 |
| `description` | string | 否 | 无 | 说明。 |
| `fields` | map | 是 | 无 | 事件字段绑定。 |

字段表达式支持三种形式：

```json
{"const": 123}
```

```json
{"ref": "ctx.instance.user"}
```

```json
{"interpolate": "user=${ctx.instance.user}"}
```

`template` 不允许声明共享策略，不判断攻击，不包含脚本。

### 4.5 `machine`

```json
{
  "id": "ssh_login_flow",
  "system": "linux_auth",
  "start": "start",
  "attack": {
    "enabled": false
  },
  "states": {
    "start": {
      "kind": "benign",
      "emit": [],
      "transitions": [
        {"to": "normal_login", "weight": 1}
      ],
      "terminal": false
    }
  }
}
```

字段：

| 字段 | 类型 | 必填 | 默认值 | 说明 |
|---|---:|---:|---:|---|
| `id` | string | 是 | 无 | machine ID。 |
| `system` | string | 是 | 无 | 所属 system ID。 |
| `start` | string | 是 | 无 | 起始 state ID。 |
| `attack` | object | 是 | `{"enabled": false}` | 默认攻击标签元信息。 |
| `states` | map | 是 | 无 | 状态集合。 |
| `description` | string | 否 | 无 | 说明。 |

`machine.attack`：

```json
{
  "enabled": true,
  "type": "ssh_bruteforce",
  "technique": "T1110",
  "tactic": "credential_access",
  "description": "optional"
}
```

`machine.attack` 只提供默认攻击标签，不描述攻击步骤或攻击链。

### 4.6 `state`

`machine.states` 使用 map，key 为 state ID：

```json
{
  "bruteforce": {
    "kind": "attack",
    "attack_label": {
      "type": "ssh_bruteforce",
      "technique": "T1110"
    },
    "emit": [
      {
        "template": "ssh_login_failed",
        "count": {"min": 20, "max": 80}
      }
    ],
    "transitions": [
      {"to": "attack_success", "weight": 1}
    ],
    "terminal": false
  }
}
```

字段：

| 字段 | 类型 | 必填 | 默认值 | 说明 |
|---|---:|---:|---:|---|
| `kind` | string | 是 | `benign` | `benign` 或 `attack`。 |
| `attack_label` | object | 否 | 无 | 当前攻击状态标签覆盖。 |
| `emit` | array | 是 | `[]` | 当前状态发射事件声明。 |
| `transitions` | array | 是 | `[]` | 状态转移候选。 |
| `terminal` | bool | 是 | `false` | 是否终止。 |
| `description` | string | 否 | 无 | 说明。 |

### 4.7 `emit`

```json
{
  "template": "ssh_login_failed",
  "count": 1
}
```

或：

```json
{
  "template": "ssh_login_failed",
  "count": {"min": 20, "max": 80}
}
```

字段：

| 字段 | 类型 | 必填 | 默认值 | 说明 |
|---|---:|---:|---:|---|
| `template` | string | 是 | 无 | template ID。 |
| `count` | int/object | 是 | `1` | 事件数量，固定整数或 `{min,max}`。 |

约束：

- 固定整数必须 `>= 0`。
- 范围形式中 `min >= 0`。
- 范围形式中 `max >= min`。

### 4.8 `transition`

```json
{
  "to": "normal_login",
  "weight": 95
}
```

字段：

| 字段 | 类型 | 必填 | 默认值 | 说明 |
|---|---:|---:|---:|---|
| `to` | string | 是 | 无 | 目标 state ID。 |
| `weight` | number | 是 | `1` | 当前状态局部分支权重。 |

约束：

- `weight > 0`。
- `weight` 是局部分支采样权重，不是全局攻击比例。

### 4.9 `instance_group`

```json
{
  "id": "ssh_users",
  "machine": "ssh_login_flow",
  "enabled": true,
  "count": 10,
  "params": {}
}
```

字段：

| 字段 | 类型 | 必填 | 默认值 | 说明 |
|---|---:|---:|---:|---|
| `id` | string | 是 | 无 | instance_group ID。 |
| `machine` | string | 是 | 无 | machine ID。 |
| `enabled` | bool | 是 | `true` | 是否启用。 |
| `count` | int | 是 | 无 | 实例数量，必须 `>= 1`。 |
| `params` | object | 否 | 无 | JSON 兼容参数。 |
| `description` | string | 否 | 无 | 说明。 |

`instance_group` 不支持 `attack_ratio`。

### 4.10 `time_profile`

```json
{
  "id": "default",
  "start": "2026-01-01T00:00:00Z",
  "timezone": "UTC",
  "granularity": "millisecond",
  "monotonic_per_instance": true,
  "system_clock_skew": {}
}
```

字段：

| 字段 | 类型 | 必填 | 默认值 | 说明 |
|---|---:|---:|---:|---|
| `id` | string | 是 | 无 | time_profile ID。 |
| `start` | string | 是 | 无 | 全局起始时间，建议 RFC3339。 |
| `timezone` | string | 是 | `UTC` | 时区名。 |
| `granularity` | string | 是 | `millisecond` | `second`、`millisecond`、`microsecond`、`nanosecond`。 |
| `monotonic_per_instance` | bool | 是 | `true` | 单实例时间是否单调。 |
| `system_clock_skew` | map | 是 | `{}` | system 级 clock skew。 |
| `description` | string | 否 | 无 | 说明。 |

clock skew 支持固定值：

```json
{"milliseconds": 100}
```

或范围值：

```json
{"min_milliseconds": -50, "max_milliseconds": 50}
```

### 4.11 `scenario`

```json
{
  "id": "ssh_bruteforce_demo",
  "systems": ["linux_auth"],
  "instance_groups": ["ssh_users"],
  "time_profile": "default"
}
```

字段：

| 字段 | 类型 | 必填 | 默认值 | 说明 |
|---|---:|---:|---:|---|
| `id` | string | 是 | 无 | scenario ID。 |
| `systems` | array | 是 | 无 | system ID 列表，至少一个。 |
| `instance_groups` | array | 是 | 无 | instance_group ID 列表，至少一个。 |
| `time_profile` | string | 是 | 无 | time_profile ID。 |
| `description` | string | 否 | 无 | 说明。 |

`scenario` 只组合对象，不编排攻击链，不建立跨系统因果。

## 5. 引用格式

### 5.1 对象 ID 引用

对象间引用统一使用字符串 ID：

| 字段 | 引用目标 |
|---|---|
| `template.system` | `systems` 中的 ID。 |
| `machine.system` | `systems` 中的 ID。 |
| `machine.states.*.emit.*.template` | `templates` 中的 ID。 |
| `machine.states.*.transitions.*.to` | 同一 `machine.states` 中的 state ID。 |
| `instance_group.machine` | `machines` 中的 ID。 |
| `scenario.systems.*` | `systems` 中的 ID。 |
| `scenario.instance_groups.*` | `instance_groups` 中的 ID。 |
| `scenario.time_profile` | `time_profiles` 中的 ID。 |

CUE schema 负责结构和基础类型；跨对象存在性由 Phase 2 Rust 静态校验负责。

### 5.2 Context 引用

Context 引用统一使用：

```text
ctx.<scope>.<name>
```

合法 scope：

```text
global
system
instance
event
```

示例：

```text
ctx.global.tenant_id
ctx.system.hostname
ctx.instance.user
ctx.event.attempt_id
```

## 6. 默认值展开规则

导出的 JSON IR 应包含以下默认值：

| 路径 | 默认值 |
|---|---:|
| `ir_version` | `"0.1"` |
| `generator.params` | `{}` |
| `context.global` | `{}` |
| `context.system` | `{}` |
| `context.instance` | `{}` |
| `context.event` | `{}` |
| `machine.attack` | `{"enabled": false}` |
| `machine.attack.enabled` | `false` |
| `state.kind` | `"benign"` |
| `state.emit` | `[]` |
| `state.transitions` | `[]` |
| `state.terminal` | `false` |
| `emit.count` | `1` |
| `transition.weight` | `1` |
| `instance_group.enabled` | `true` |
| `time_profile.timezone` | `"UTC"` |
| `time_profile.granularity` | `"millisecond"` |
| `time_profile.monotonic_per_instance` | `true` |
| `time_profile.system_clock_skew` | `{}` |

如果某个字段在 CUE 源文件中未显式声明，但有默认值，`cue export` 后的 JSON 应看到该默认值。

## 7. 明确禁止的字段和对象

IR v0.1 不支持：

- `entity`
- `attack_chain`
- `attack_ratio`
- `stage`
- 跨实例依赖字段
- 跨系统依赖字段
- 事件级 DAG 字段
- 脚本字段
- hook 字段
- 外部插件字段

尤其不要把这些字段藏进 `scenario`、`instance_group`、`template` 或 `machine.attack`。这种做法像把地下室改成核反应堆，图纸上没写，不代表不会炸。

## 8. 导出与校验

推荐流程：

```bash
scripts/validate-cue.sh
scripts/export-ir.sh
```

`export-ir.sh` 应生成 JSON，并使用根 Schema `#IR` 对导出的 JSON 再做一次校验，确保最终 IR 顶层没有越界字段。

## 9. 最小完整 IR 示例

```json
{
  "ir_version": "0.1",
  "systems": {
    "linux_auth": {
      "id": "linux_auth",
      "kind": "linux_auth"
    }
  },
  "generators": {
    "username": {
      "id": "username",
      "type": "username",
      "params": {}
    }
  },
  "context": {
    "global": {},
    "system": {
      "linux_auth": {
        "hostname": {
          "source": {"const": "host-1"}
        }
      }
    },
    "instance": {
      "ssh_login_flow": {
        "user": {
          "source": {"generator": "username"}
        }
      }
    },
    "event": {}
  },
  "templates": {
    "ssh_login_success": {
      "id": "ssh_login_success",
      "system": "linux_auth",
      "fields": {
        "user": {"ref": "ctx.instance.user"},
        "host": {"ref": "ctx.system.hostname"},
        "result": {"const": "success"}
      }
    }
  },
  "machines": {
    "ssh_login_flow": {
      "id": "ssh_login_flow",
      "system": "linux_auth",
      "start": "start",
      "attack": {"enabled": false},
      "states": {
        "start": {
          "kind": "benign",
          "emit": [],
          "transitions": [
            {"to": "success", "weight": 1}
          ],
          "terminal": false
        },
        "success": {
          "kind": "benign",
          "emit": [
            {"template": "ssh_login_success", "count": 1}
          ],
          "transitions": [],
          "terminal": true
        }
      }
    }
  },
  "instance_groups": {
    "ssh_users": {
      "id": "ssh_users",
      "machine": "ssh_login_flow",
      "enabled": true,
      "count": 1
    }
  },
  "time_profiles": {
    "default": {
      "id": "default",
      "start": "2026-01-01T00:00:00Z",
      "timezone": "UTC",
      "granularity": "millisecond",
      "monotonic_per_instance": true,
      "system_clock_skew": {}
    }
  },
  "scenario": {
    "id": "minimal_demo",
    "systems": ["linux_auth"],
    "instance_groups": ["ssh_users"],
    "time_profile": "default"
  }
}
```
