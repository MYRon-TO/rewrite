# 03. 单 machine 攻击分支建模

## 1. 设计结论

当前版本采用“超级简化攻击建模”：

> 攻击仅存在于单个 `machine` 内，表现为状态机中的攻击状态与攻击转移分支；攻击样本比例主要由 `machine.states[].transitions[].weight` 控制。

不引入：

* 顶层 `attack_chain`。
* 跨 `machine` 攻击链。
* 跨 `system` 攻击因果。
* 攻击阶段 `stage`。
* 跨实例身份图或 `entity`。
* 事件级 DAG。
* 脚本式攻击编排。
* `instance_group.attack_ratio`。

## 2. 建模目标

支持在一个 `machine` 内同时表达：

1. 正常路径。
2. 攻击路径。
3. 攻击路径进入概率。
4. 攻击状态发出的日志。
5. 可用于训练或评估的攻击标签。

## 3. 设计边界

攻击路径只是状态机的一个分支：

```text
machine
  states
    start
    normal_login
    bruteforce
    attack_success
  transitions
    start -> normal_login
    start -> bruteforce
```

这不是攻击链编排器。不要把跨系统入侵、横向移动、提权、数据外传全塞进第一版。那会把一个清爽状态机塞成一锅安全运营麻辣烫。

## 4. `machine.attack`

`machine.attack` 提供默认攻击标签元信息。

```cue
attack?: {
  enabled: bool | *false
  type?: string
  technique?: string
  tactic?: string
  description?: string
}
```

字段语义：

1. `enabled`：该 `machine` 是否包含攻击分支。
2. `type`：默认攻击类型，例如 `ssh_bruteforce`、`web_scan`。
3. `technique`：默认 ATT&CK technique ID，例如 `T1110`。
4. `tactic`：默认 ATT&CK tactic，例如 `credential_access`。
5. `description`：人类可读说明。

约束：

* 当任一状态的 `kind` 为 `attack` 时，建议 `machine.attack.enabled == true`。
* 当 `machine.attack.enabled == false` 时，不应出现 `kind: "attack"` 的状态。
* `machine.attack` 不描述攻击执行流程，只提供标签元信息。

## 5. `state.kind`

`state.kind` 标记状态行为类别。

```cue
kind?: "benign" | "attack" | *"benign"
```

语义：

* `benign`：该状态发出的事件默认不是攻击。
* `attack`：该状态发出的事件默认是攻击。

约束：

* `kind` 不改变状态机执行语义。
* `kind` 只影响标签和语义解释。
* 缺省时按 `benign` 处理。

## 6. `state.attack_label`

`state.attack_label` 对当前攻击状态的标签做覆盖。

```cue
attack_label?: {
  type?: string
  technique?: string
  tactic?: string
  name?: string
}
```

语义：

* 当不同攻击状态对应不同攻击类型或技术编号时使用。
* 优先级高于 `machine.attack`。

约束：

* 只应出现在 `kind: "attack"` 的状态上。
* 不承担流程编排职责。

## 7. `transition.weight` 如何控制攻击路径

状态转移建议结构：

```cue
transitions: [...{
  to: string
  weight?: number | *1
}]
```

规则：

1. 同一状态下所有候选转移按 `weight` 归一化采样。
2. `weight` 控制进入攻击路径的局部分支概率。
3. `weight` 不是全局攻击比例保证。
4. 多状态、多循环下的最终攻击样本比例由状态机结构和权重共同决定。

示例：

```cue
transitions: [
  {to: "normal_login", weight: 95},
  {to: "bruteforce", weight: 5},
]
```

含义：在当前状态下，进入 `bruteforce` 分支的局部概率约为 5%。

必须写清楚的一句话：

> `transition.weight` 控制的是局部分支概率，不是整个数据集最终攻击占比的硬约束。

## 8. `emit.count` 如何表达爆发型事件

状态发射事件建议结构：

```cue
emit?: [...{
  template: string
  count?: int | {
    min: int
    max: int
  } | *1
}]
```

规则：

1. `count` 用于表达一个状态可发出多条同类事件。
2. 适合爆破、扫描、枚举等第一版攻击样式。
3. 不在 `emit` 中引入复杂循环、条件或脚本。

示例：

```cue
emit: [{
  template: "ssh_login_failed"
  count: {min: 20, max: 80}
}]
```

## 9. 标签生成规则

定义层只声明标签规则，不实现输出逻辑。

建议事件标签字段：

```cue
label: {
  is_attack: bool
  attack_type?: string
  technique?: string
  tactic?: string
  attack_state?: string
  attack_machine?: string
}
```

生成规则：

1. 如果事件由 `kind: "attack"` 的状态发出，则 `is_attack = true`。
2. 如果事件由 `kind: "benign"` 的状态发出，则 `is_attack = false`。
3. `attack_type` 优先取 `state.attack_label.type`，否则取 `machine.attack.type`。
4. `technique` 优先取 `state.attack_label.technique`，否则取 `machine.attack.technique`。
5. `tactic` 优先取 `state.attack_label.tactic`，否则取 `machine.attack.tactic`。
6. `attack_state` 记录发出事件的攻击状态 ID。
7. `attack_machine` 记录所属 `machine` ID。

## 10. 模板与攻击语义解耦

`template` 不需要知道自己是否攻击。

同一模板可以被正常状态和攻击状态复用。攻击标签由发出模板的 `state.kind` 决定，而不是模板固化决定。

例子：

* 正常状态发出 `ssh_login_failed`：用户输错密码，`is_attack = false`。
* 攻击状态发出 `ssh_login_failed`：爆破失败日志，`is_attack = true`。

## 11. 完整 SSH 爆破示例

```cue
machine: {
  id: "ssh_login_flow"
  system: "linux_auth"
  start: "start"

  attack: {
    enabled: true
    type: "ssh_bruteforce"
    technique: "T1110"
    tactic: "credential_access"
  }

  states: {
    start: {
      kind: "benign"
      transitions: [
        {to: "normal_login", weight: 95},
        {to: "bruteforce", weight: 5},
      ]
    }

    normal_login: {
      kind: "benign"
      emit: [
        {template: "ssh_login_success"},
      ]
      transitions: [
        {to: "normal_activity", weight: 1},
      ]
    }

    normal_activity: {
      kind: "benign"
      emit: [
        {template: "shell_command"},
      ]
      terminal: true
    }

    bruteforce: {
      kind: "attack"
      attack_label: {
        type: "ssh_bruteforce"
        technique: "T1110"
      }
      emit: [
        {
          template: "ssh_login_failed"
          count: {min: 20, max: 80}
        },
      ]
      transitions: [
        {to: "attack_success", weight: 1},
      ]
    }

    attack_success: {
      kind: "attack"
      attack_label: {
        type: "valid_account_login"
        technique: "T1078"
      }
      emit: [
        {template: "ssh_login_success"},
      ]
      transitions: [
        {to: "suspicious_activity", weight: 1},
      ]
    }

    suspicious_activity: {
      kind: "attack"
      emit: [
        {
          template: "sudo_attempt"
          count: {min: 1, max: 5}
        },
      ]
      terminal: true
    }
  }
}
```

## 12. 静态校验建议

后续如果编写 CUE schema，可加入：

1. `state.kind` 默认为 `benign`。
2. `attack_label` 仅允许用于 `kind: "attack"` 的状态。
3. `machine.attack.enabled == false` 时禁止出现攻击状态。
4. `transition.weight > 0`。
5. `emit.count.min <= emit.count.max`。
6. `emit.count.min >= 0`。
