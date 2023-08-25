# Enmeshed Monorepo

This a template to setup the enmeshed system as a monorepo, this is especially usefull for implementing new features.

## Benefits in comparison to earlier Monorepo template

- No template package.json, this was a problem since a clone would not override it and therefore the dependencies are not up to date.
- No zsh required this implementation does not need a special shell and runs on windows and linux
- No need for jq

# Prerequisites

- [Rust]("https://www.rust-lang.org/tools/install")
- [node including npm](https://nodejs.org/en/download)
- pnpm (npm i pnpm)

# Setup

1. npm run setup (this clones all repos and updates their package.json to use the workspace version)
2. pnpm i
3. npm run build
