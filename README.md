# 🌠 流星雨记账

Windows 桌面个人记账应用，记录每一笔人民币支出，两级分类（一级大类 + 二级小类）。

## 项目文档

- 产品需求、协作规则、分类设计、技术选型、里程碑详见 [CLAUDE.md](CLAUDE.md)
- 用户使用说明详见 [使用说明.md](使用说明.md)

## 技术栈

- 桌面框架：Tauri 2（Rust）
- 界面：Vue 3 + Element Plus + TypeScript
- 数据：SQLite（本地单文件，位于系统应用数据目录）
- 图表：ECharts

## 开发

```bash
npm install          # 安装界面依赖
npm run tauri dev    # 启动开发模式（窗口运行）
npm run tauri build  # 打包 Windows 安装程序
```

## 里程碑

- M1 记一笔 + 账单列表 + 数据存取
- M2 分类管理 + 月度汇总 + 筛选搜索
- M3 统计图表 + 数据导出
- M4 打包安装程序 + 验收
