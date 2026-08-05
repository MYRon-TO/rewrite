# 用户文档

## 1. 这个工具做什么

`rewrite` 的输入不是手写 JSONL，而是 CUE 场景定义。典型流程是：

```text
CUE definitions
  -> cue vet / cue export
  -> generated JSON IR
  -> rewrite validate
  -> rewrite generate
  -> JSON Lines logs
```

当前 MVP 的边界：

- 支持单系统、多实例、多系统并行生成。
- 支持状态机、模板渲染、上下文变量、时间推进、攻击标签。
- Rust 运行时只读取已导出的 JSON IR，不直接调用 CUE。
- 不支持跨系统因果、跨实例依赖、`entity`、`attack_chain`、`attack_ratio`。别把这些东西塞进 MVP，塞进去就像给电动车装火箭喷口，响是挺响，能不能活另说。

## 2. 安装依赖

### 2.1 Rust

安装 Rust toolchain：

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

验证：

```bash
rustc --version
cargo --version
```

### 2.2 CUE CLI

任选一种安装方式。

macOS：

```bash
brew install cue-lang/tap/cue
```

通过 Go：

```bash
go install cuelang.org/go/cmd/cue@latest
```

验证：

```bash
cue version
```

如果 `cue` 找不到，通常是 Go 的 bin 目录没有进入 `PATH`，检查：

```bash
go env GOPATH
```

然后把 `$(go env GOPATH)/bin` 加到 `PATH`。

## 3. 从零跑通示例

以下命令默认在仓库根目录执行。

### 3.1 校验 CUE 示例

```bash
make cue-validate
```

等价于调用：

```bash
scripts/validate-cue.sh
```

脚本会校验 `cue/examples/*.cue`，并在存在 `cue/schema/*.cue` 时把 schema 一起纳入校验。

### 3.2 导出 JSON IR

```bash
make cue-export
```

等价于调用：

```bash
scripts/export-ir.sh
```

默认输出：

```text
generated/examples/ssh_bruteforce.json
generated/examples/multi_instance_login.json
generated/examples/multi_system.json
```

只导出某一个例子：

```bash
scripts/export-ir.sh cue/examples/ssh_bruteforce.cue
```

### 3.3 静态验证 IR

```bash
cargo run -- validate --ir generated/examples/ssh_bruteforce.json
```

成功输出：

```text
IR validation passed: generated/examples/ssh_bruteforce.json
```

这一步只检查 IR，不生成日志。它会拦截常见问题，例如：

- 引用不存在：场景引用了不存在的 system、machine、template。
- 状态机非法：start 状态不存在、transition 指向不存在状态。
- MVP 边界漂移：出现 `entity`、`attack_chain`、`attack_ratio` 等禁止字段。

如果 IR 中的 `corpus_choice` 使用相对 `source.path`，路径默认相对于 IR 文件所在目录；也可显式指定本地资产根：

```bash
cargo run -- validate --ir generated/examples/example.json --asset-dir ./assets
```

绝对语料路径直接使用。外部文件必须是 UTF-8，最大 16 MiB、最多 1,000,000 行、每行最多 4096 个 Unicode 字符。仅移除 LF/CRLF，不 trim，`#` 不表示注释。
### 3.4 生成日志

```bash
cargo run -- generate \
  --ir generated/examples/ssh_bruteforce.json \
  --out out/events.jsonl \
  --seed 42
```

成功输出类似：

```text
Generated 102 events from generated/examples/ssh_bruteforce.json to out/events.jsonl with seed 42
```

说明：

- `--ir`：输入的 JSON IR 文件。
- `--out`：输出 JSONL 文件路径；父目录不存在时会自动创建。
- `--seed`：随机种子；相同 IR 与相同 seed 会产生可复现输出。

使用外部语料时，`generate` 同样接受 `--asset-dir <PATH>`。资源首次使用时按规范化数据源缓存原始行；过滤配置和 `shuffle_cycle` 抽样状态彼此隔离。相同 IR、语料内容、seed 与调用顺序可复现；语料内容或调用顺序变化时不保证相同输出。内置 `human-passwords-v1` 来自随程序打包的 `data/human_password.txt`。

`corpus_choice` 默认 `min_length=1`、`max_length=128`、`sampling=uniform`、`deduplicate=true`、`keep_empty=false`。`machine_secret` 生成的密码、Token、JWT、PEM、云凭据等仅用于仿真，不具备密码学安全性或真实性。两类生成器都不会自动设置攻击标签；是否为攻击事件仍由 machine state 和现有攻击标签配置决定。
也可以一条命令跑默认示例：

```bash
make run-example
```

默认会生成：

```text
out/events.jsonl
```

## 4. 查看输出

查看前几条日志：

```bash
head -n 3 out/events.jsonl
```

每一行都是一条完整 JSON 事件。典型结构：

```json
{
  "timestamp": "2026-05-17T00:00:00Z",
  "scenario": "ssh_bruteforce_demo",
  "system": "linux_auth",
  "instance": "ssh_login_sessions-0001",
  "machine": "ssh_login_flow",
  "state": "normal_login",
  "template": "ssh_login_success",
  "fields": {
    "host": "ssh-gateway-01",
    "program": "sshd",
    "result": "success"
  },
  "label": {
    "is_attack": false
  }
}
```

字段含义：

| 字段 | 含义 |
| --- | --- |
| `timestamp` | 事件时间戳。 |
| `scenario` | 场景 ID。 |
| `system` | 系统 ID。 |
| `instance` | 展开后的实例 ID。 |
| `machine` | 状态机 ID。 |
| `state` | 产生该事件的状态 ID。 |
| `template` | 使用的模板 ID。 |
| `fields` | 模板渲染后的业务日志字段。 |
| `label` | 攻击标签，始终包含 `is_attack`。 |

查找攻击事件：

```bash
python - <<'PY'
import json
with open('out/events.jsonl') as f:
    for line_no, line in enumerate(f, 1):
        event = json.loads(line)
        if event['label'].get('is_attack'):
            print(line_no, json.dumps(event, ensure_ascii=False))
            break
PY
```

攻击事件的 `label` 可能包含：

```json
{
  "is_attack": true,
  "attack_type": "credential_access",
  "technique": "T1110",
  "tactic": "Credential Access",
  "attack_state": "bruteforce",
  "attack_machine": "ssh_login_flow"
}
```

## 5. 使用其他内置示例

多实例登录：

```bash
cargo run -- generate \
  --ir generated/examples/multi_instance_login.json \
  --out out/multi_instance_login.jsonl \
  --seed 42
```

多系统并行：

```bash
cargo run -- generate \
  --ir generated/examples/multi_system.json \
  --out out/multi_system.jsonl \
  --seed 42
```

这些示例用于验证：

- 多个 instance 共享 global/system context，但不共享 instance context。
- 多个 system 可并行生成事件，但不建立跨系统因果依赖。

## 6. 常见问题

### 6.1 `CUE CLI not found`

原因：没有安装 CUE，或 `cue` 不在 `PATH`。

处理：

```bash
cue version
```

如果失败，重新安装 CUE，并确认可执行文件目录已加入 `PATH`。

### 6.2 `unsupported IR format` 或版本错误

原因：CLI 只接受扩展名为 `.json` 且 `ir_version` 为 `"0.2"` 的 JSON IR；`0.1` 不再读取。

处理：先运行：

```bash
make cue-export
```

然后使用 `generated/examples/*.json`。

### 6.3 生成结果和文档里的数量不同

先确认 seed 是否一致：

```bash
cargo run -- generate --ir generated/examples/ssh_bruteforce.json --out out/events.jsonl --seed 42
```

如果 IR 或示例定义被修改，事件数量可能变化，这是正常的。

### 6.4 想手动创建新场景

建议先复制现有示例：

```bash
cp cue/examples/ssh_bruteforce.cue cue/examples/my_scenario.cue
```

然后按流程执行：

```bash
scripts/validate-cue.sh cue/examples/my_scenario.cue
scripts/export-ir.sh cue/examples/my_scenario.cue
cargo run -- validate --ir generated/examples/my_scenario.json
cargo run -- generate --ir generated/examples/my_scenario.json --out out/my_scenario.jsonl --seed 42
```

如果新增字段被 drift 校验拒绝，先确认它是否属于 MVP 支持范围。不要把未来版本的对象偷偷塞进当前 IR，不然报错不是工具小心眼，是你把边界当橡皮筋。

## 7. 质量门禁

提交前建议运行：

```bash
make check
```

它会依次执行：

1. CUE 示例校验。
2. 非法 CUE 用例拒绝测试。
3. CUE 导出 IR。
4. `cargo check`。
5. `cargo test`。
