# Enmeshed Monorepo

This is a template to setup the enmeshed system as a monorepo, this is especially usefull for implementing new features.

## Benefits in comparison to earlier Monorepo template

- No template package.json, this was a problem since a clone would not override it and therefore the dependencies are not up to date.
- No zsh required this implementation does not need a special shell and runs on windows and linux
- No need for jq
- no need for pnpm
- no need for nx

# Prerequisites

- [Rust]("https://www.rust-lang.org/tools/install")
- [node including npm](https://nodejs.org/en/download)
- typescript (npm i typescript -g)
- yarn (npm i yarn)

# Setup

1. (in monosetup) cargo r -r -- -c init (this clones all repos and updates their package.json to use the workspace version)
2. yarn
3. yarn workspaces run build:notest 
