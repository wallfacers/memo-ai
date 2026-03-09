# 进行中的执行计划

本目录存放当前正在执行的开发计划。

## 命名规范

文件名格式：`NN-{描述}.md`（顺序编号，与已完成计划共享序列）

示例：`09-unified-stage-progress-style.md`

## 计划模板

```markdown
# 计划标题

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 本次计划要完成什么

**Architecture:** 方案简述

**Tech Stack:** 相关技术栈

---

## Task 1: 任务名称

**Files:**
- Create/Modify: `路径`

**Step 1: ...**

**Step 2: 验证编译/类型检查**

**Step 3: Commit**
```

---

## 当前计划

| 计划 | 目标 |
|------|------|
| [09-unified-stage-progress-style.md](./09-unified-stage-progress-style.md) | 提取 StageProgressList 共享组件，统一 Pipeline 和 SummaryTab 两处进度条视觉风格 |
