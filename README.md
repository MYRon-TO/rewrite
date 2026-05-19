# rewrite

`rewrite` 是一个日志生成器 MVP：使用 CUE 定义场景并导出 JSON IR，再由 Rust CLI 读取 IR、执行状态机，并输出带攻击标签的 JSON Lines 日志。

## 快速开始

### 1. 准备环境

需要安装：

- Rust toolchain：<https://www.rust-lang.org/tools/install>
- CUE CLI：<https://cuelang.org/docs/introduction/installation/>

可选安装方式：

```bash
# macOS
brew install cue-lang/tap/cue

# 或通过 Go 安装
# go install cuelang.org/go/cmd/cue@latest
```

检查版本：

```bash
rustc --version
cargo --version
cue version
```

### 2. 校验 CUE 定义并导出 IR

```bash
make cue-validate
make cue-export
```

导出的 IR 默认位于：

```text
generated/examples/*.json
```

### 3. 验证 IR

```bash
cargo run -- validate --ir generated/examples/ssh_bruteforce.json
```

成功时会输出类似：

```text
IR validation passed: generated/examples/ssh_bruteforce.json
```

### 4. 生成日志

默认生成完整 JSONL 事件：

```bash
cargo run -- generate \
  --ir generated/examples/ssh_bruteforce.json \
  --out out/events.jsonl \
  --seed 42
```

也可以生成精简 CSV，只包含输出日志字段和攻击标记：

```bash
cargo run -- generate \
  --ir generated/examples/ssh_bruteforce.json \
  --out out/events.csv \
  --format csv \
  --seed 42
```

也可以直接使用 Makefile：

```bash
make run-example
```

### 5. 查看输出

```bash
head -n 3 out/events.jsonl
```

每行是一条 JSON 事件，结构包含时间、场景、系统、实例、状态、模板字段和攻击标签：

```json
{
  "timestamp": "2026-05-17T00:00:00Z",
  "scenario": "ssh_bruteforce_demo",
  "system": "linux_auth",
  "instance": "ssh_login_sessions-0001",
  "machine": "ssh_login_flow",
  "state": "normal_login",
  "template": "ssh_login_success",
  "fields": {},
  "label": { "is_attack": false }
}
```

攻击事件会带有 `label.is_attack=true`，并可能包含 `attack_type`、`technique`、`tactic`、`attack_state`、`attack_machine` 等字段。

## 常用命令

```bash
make check          # CUE 校验、非法用例校验、IR 导出、cargo check、cargo test
make test           # 运行 Rust 测试
make cue-validate   # 校验 cue/examples/*.cue
make cue-export     # 导出 generated/examples/*.json
make run-example    # 导出 IR 并生成 out/events.jsonl
```

CLI 子命令：

```bash
cargo run -- validate --ir <path-to-ir.json>
cargo run -- generate --ir <path-to-ir.json> --out <path-to-events.jsonl> [--seed <u64>]
```

## 示例场景

当前仓库包含：

- `cue/examples/ssh_bruteforce.cue`：SSH 正常登录与爆破攻击。
- `cue/examples/multi_instance_login.cue`：多实例登录样例。
- `cue/examples/multi_system.cue`：多系统并行样例。

更多使用说明见 [`docs/usage.md`](docs/usage.md)。
