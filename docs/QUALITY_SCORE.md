# 代码质量标准

## Rust 后端

### 错误处理

- 所有可失败操作返回 `Result<T, AppError>`，不使用 `unwrap()` / `expect()`（测试代码除外）
- 错误类型定义在 `src-tauri/src/error.rs`，使用 `thiserror` crate
- Tauri 命令中的错误通过 `AppError` 序列化后传递给前端

### 代码组织

- 每个模块（audio、asr、llm、db）有独立的 `mod.rs` 作为公开接口
- 函数长度不超过 50 行，超过则提取子函数
- 避免深度嵌套（最多 3 层），使用 `?` 操作符早返回

## TypeScript 前端

### 类型安全

- 禁止使用 `any`，使用 `unknown` 或具体类型
- Tauri 命令返回值必须有类型定义（在 `src/types/index.ts`）
- `null` 和 `undefined` 需显式处理，不使用非空断言 `!`

### 组件质量

- 组件不超过 200 行
- 副作用（fetch、订阅）必须在 `useEffect` 中，有清理函数
- 避免 prop drilling 超过 2 层，改用 Zustand

## 通用标准

### 命名

- 变量/函数：`camelCase`（TS）/ `snake_case`（Rust）
- 常量：`UPPER_SNAKE_CASE`
- 类型/接口：`PascalCase`
- 文件名：与主要导出名称一致

### 不允许的模式

- 魔法数字（使用命名常量）
- 注释掉的代码（直接删除）
- TODO 注释（转为 tech-debt-tracker.md 条目）
- 空 catch 块

## Code Review 检查点

1. 新功能有对应的错误处理
2. 数据库操作有事务保护（多步操作）
3. Tauri 命令参数有基本验证
4. 没有引入不必要的依赖
