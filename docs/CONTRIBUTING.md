# Contributing to AtlasRemote

感谢你对 AtlasRemote 的贡献！

## 开发环境

1. 安装 Rust: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
2. 安装 Windows SDK
3. 克隆仓库: git clone https://github.com/Zhiyilang074811/AtlasRemote.git
4. 进入目录: cd AtlasRemote
5. 构建: cargo build --workspace

## 代码规范

- 遵循 [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- 提交信息使用 [Conventional Commits](https://www.conventionalcommits.org/)
- 测试: cargo test --workspace

## 提交 PR

1. Fork 本仓库
2. 创建分支: git checkout -b feat/your-feature
3. 提交更改: git commit -m 'feat: add xxx'
4. 推送: git push origin feat/your-feature
5. 创建 Pull Request
