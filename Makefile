.PHONY: app build check checks clean dmg docs-check docs-impact fmt icon lint open release run test

build:
	cargo build --workspace

release:
	cargo build --release --workspace --locked

run:
	cargo run -p space-invaders-app

test:
	cargo test --workspace --locked

checks:
	cargo run --release --locked -p space-invaders-checks

lint:
	cargo clippy --workspace --all-targets --locked -- -D warnings

fmt:
	cargo fmt --all

check:
	./scripts/check.sh

docs-check:
	python3 scripts/check_docs.py

docs-impact:
	python3 scripts/check_docs.py --changed

app:
	./scripts/build_app.sh

open: app
	open dist/SPACE-INVADERS.app

dmg: app
	hdiutil create -volname "SPACE INVADERS" -srcfolder "dist/SPACE-INVADERS.app" -ov -format UDZO "dist/SPACE-INVADERS-1.0.dmg"

icon:
	python3 scripts/make_icon.py

clean:
	cargo clean
	rm -rf dist
