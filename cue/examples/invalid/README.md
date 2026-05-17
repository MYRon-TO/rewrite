# Phase 1 invalid CUE samples

这些文件用于手动或测试脚本验证定义层边界，不会被默认 `scripts/validate-cue.sh` 扫描，因为默认只读取 `cue/examples/*.cue`。

单个验证示例：

```bash
scripts/validate-cue.sh cue/examples/invalid/transition_weight_zero.cue
```

预期结果：每个 `.cue` 文件都应该校验失败。
