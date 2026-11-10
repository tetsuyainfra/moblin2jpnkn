# 開発手順

## バージョンを上げてリリースする

リリースには `Cargo.toml` のパッケージバージョンと、それに対応する `v` 接頭辞付き Git tag を使います。`v*` タグを push すると GitHub Actions が起動し、amd64/arm64 向け Debian パッケージをビルドして GitHub Release を作成します。

1. 変更内容に応じて次の SemVer バージョンを決めます。まだ `0.x` の間は、互換性を保証しない開発段階であることも考慮してください。
2. `Cargo.toml` の `[package]` にある `version` を更新し、Cargo が `Cargo.lock` の `moblin2jpnkn` のバージョンを同期するようにします。

	```sh
	cargo check
	```

	`Cargo.lock` に意図しない依存更新が入っていないことも確認します。
3. テストとパッケージ作成を確認します。

	```sh
	cargo test --locked
	cargo install cargo-deb # 初回のみ
	cargo deb
	```

4. バージョンが一致していることを確認し、`Cargo.toml` と `Cargo.lock` の変更をコミットします。例では `0.0.5` を次のバージョンとします。

	```sh
	git diff -- Cargo.toml Cargo.lock
	git add Cargo.toml Cargo.lock
	git commit -m "chore: release v0.0.5"
	```

5. そのコミットに注釈付きタグを付け、ブランチとタグをそれぞれ push します。タグ名は `Cargo.toml` のバージョンと一致させます。

	```sh
	git tag -a v0.0.5 -m "v0.0.5"
	git push origin main
	git push origin v0.0.5
	```

6. GitHub Actions の `build` ワークフローが成功したことを確認します。成功すると `.deb` パッケージ、`SHA256SUMS` とその署名を添付した GitHub Release が作られます。

リリースジョブはリポジトリの Actions Secrets に `GPG_PRIVATE_KEY` と `GPG_PASSPHRASE` が設定されていることを前提とします。公開済みのバージョンタグは付け替えず、修正が必要な場合は新しいバージョンをリリースしてください。

